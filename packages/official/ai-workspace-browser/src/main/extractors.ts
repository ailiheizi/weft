import { Readability } from '@mozilla/readability';
import { JSDOM } from 'jsdom';
import TurndownService from 'turndown';

export interface ExtractedMarkdown {
  title: string;
  text: string;
}

const turndown = new TurndownService({
  headingStyle: 'atx',
  codeBlockStyle: 'fenced',
  bulletListMarker: '-'
});

turndown.addRule('compactLinks', {
  filter: 'a',
  replacement(content, node) {
    const href = node instanceof HTMLElement ? node.getAttribute('href') : null;
    const label = content.trim();
    if (!href || !label) return label;
    return `[${label}](${href})`;
  }
});

export function extractMarkdownFromHtml(html: string, url: string, fallbackTitle: string): ExtractedMarkdown {
  const dom = new JSDOM(html, { url });
  const document = dom.window.document;
  const article = new Readability(document).parse();

  if (article?.content) {
    return {
      title: article.title || fallbackTitle,
      text: turndown.turndown(article.content).slice(0, 24000)
    };
  }

  const fallbackRoot = document.querySelector('main, article, [role="main"]') ?? document.body;
  return {
    title: document.title || fallbackTitle,
    text: turndown.turndown(fallbackRoot?.innerHTML ?? '').slice(0, 24000)
  };
}
