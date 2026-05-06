import { wrap_prompt_text } from './battle-ui-draw';

describe('battle-ui-draw wrap_prompt_text', () => {
  it('wraps prompt text using word boundaries and long-word splits', () => {
    const lines = wrap_prompt_text('What will TOTODILE do?', 7, 4);
    expect(lines).toEqual(['What', 'will TO', 'TODILE', 'do?']);
  });
});
