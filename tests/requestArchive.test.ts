import { describe, expect, it } from 'bun:test';
import {
  archiveReadiness,
  formatArchiveBytes,
  formatArchiveJson,
  parseArchiveMessages,
  parseArchiveToolNames,
} from '../src/services/requestArchive';

describe('archiveReadiness', () => {
  it('未启用时报告 disabled', () => {
    expect(archiveReadiness({ settingsEnabled: false, requestLogEnabled: true })).toBe('disabled');
  });

  it('启用但核心未开启 request-log 时提示需要开启', () => {
    expect(archiveReadiness({ settingsEnabled: true, requestLogEnabled: false })).toBe(
      'awaiting-request-log',
    );
  });

  it('两者都开启时为 ready', () => {
    expect(archiveReadiness({ settingsEnabled: true, requestLogEnabled: true })).toBe('ready');
  });
});

describe('formatArchiveBytes', () => {
  it('按二进制单位换算并处理无效值', () => {
    expect(formatArchiveBytes(0)).toBe('0 B');
    expect(formatArchiveBytes(-10)).toBe('0 B');
    expect(formatArchiveBytes(512)).toBe('512 B');
    expect(formatArchiveBytes(1536)).toBe('1.5 KB');
    expect(formatArchiveBytes(5 * 1024 * 1024 * 1024)).toBe('5.0 GB');
  });
});

describe('parseArchiveMessages', () => {
  it('解析 OpenAI chat completions 的消息与工具调用', () => {
    const messages = parseArchiveMessages(
      JSON.stringify([
        { role: 'system', content: 'be terse' },
        { role: 'user', content: 'hello' },
        {
          role: 'assistant',
          content: null,
          tool_calls: [{ function: { name: 'read_file' } }],
        },
      ]),
    );

    expect(messages).toHaveLength(3);
    expect(messages[0]).toEqual({ role: 'system', text: 'be terse', toolCalls: [] });
    expect(messages[1].text).toBe('hello');
    expect(messages[2].toolCalls).toEqual(['read_file']);
  });

  it('解析 Claude 内容块与 tool_use', () => {
    const messages = parseArchiveMessages(
      JSON.stringify([
        {
          role: 'user',
          content: [
            { type: 'text', text: 'first' },
            { type: 'text', text: 'second' },
          ],
        },
        { role: 'assistant', content: [{ type: 'tool_use', name: 'bash' }] },
      ]),
    );

    expect(messages[0].text).toBe('first\nsecond');
    expect(messages[1].toolCalls).toEqual(['bash']);
  });

  it('解析 Gemini contents 的 parts 与 functionCall', () => {
    const messages = parseArchiveMessages(
      JSON.stringify([
        { role: 'user', parts: [{ text: 'gemini hello' }] },
        { role: 'model', parts: [{ functionCall: { name: 'search' } }] },
      ]),
    );

    expect(messages[0].text).toBe('gemini hello');
    expect(messages[1].toolCalls).toEqual(['search']);
  });

  it('对空值和非法 JSON 返回空数组', () => {
    expect(parseArchiveMessages('')).toEqual([]);
    expect(parseArchiveMessages('not json')).toEqual([]);
    expect(parseArchiveMessages('{"messages":[]}')).toEqual([]);
  });
});

describe('parseArchiveToolNames', () => {
  it('支持 OpenAI、Claude 与 Gemini 三种工具声明形态', () => {
    expect(
      parseArchiveToolNames(JSON.stringify([{ type: 'function', function: { name: 'get_time' } }])),
    ).toEqual(['get_time']);
    expect(parseArchiveToolNames(JSON.stringify([{ name: 'read_file' }]))).toEqual(['read_file']);
    expect(
      parseArchiveToolNames(JSON.stringify({ functionDeclarations: [{ name: 'lookup' }] })),
    ).toEqual(['lookup']);
  });

  it('对空值和非法 JSON 返回空数组', () => {
    expect(parseArchiveToolNames('')).toEqual([]);
    expect(parseArchiveToolNames('[[[')).toEqual([]);
  });
});

describe('formatArchiveJson', () => {
  it('格式化合法 JSON 并原样返回非 JSON 文本', () => {
    expect(formatArchiveJson('{"a":1}')).toBe('{\n  "a": 1\n}');
    expect(formatArchiveJson('data: [DONE]')).toBe('data: [DONE]');
    expect(formatArchiveJson('   ')).toBe('');
  });
});
