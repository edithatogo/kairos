import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('./build.js', import.meta.url), 'utf8');
const renderStart = source.indexOf('function renderPage');
const start = source.indexOf('<script>\n(function() {', renderStart);
assert.notEqual(start, -1, 'search client script exists in generated page template');
const scriptStart = source.indexOf('\n', start) + 1;
const scriptEnd = source.indexOf('</script>', scriptStart);
assert.notEqual(scriptEnd, -1, 'search client script is closed');

class Element {
  constructor(tagName) {
    this.tagName = tagName;
    this.children = [];
    this.childNodes = this.children;
    this.listeners = {};
    this.hidden = false;
    this.value = '';
    this.className = '';
    this.textContent = '';
  }

  appendChild(child) {
    this.children.push(child);
    return child;
  }

  replaceChildren(...children) {
    this.children = children;
    this.childNodes = this.children;
  }

  addEventListener(name, callback) {
    this.listeners[name] = callback;
  }
}

const input = new Element('input');
const results = new Element('div');
const document = {
  createElement: (tag) => new Element(tag),
  getElementById: (id) => id === 'search' ? input : id === 'search-results' ? results : null,
  addEventListener: () => {},
};
const origin = 'https://docs.example.test';
const context = {
  document,
  window: {
    location: { href: `${origin}/kairos/docs/search.html`, origin },
    searchIndex: [
      { title: '<img src=x onerror=alert(1)>', excerpt: '<svg onload=alert(2)> excerpt', headings: [], href: '/kairos/docs/safe.html' },
      { title: 'javascript link', excerpt: 'scheme attack', headings: [], href: 'javascript:alert(3)' },
      { title: 'external link', excerpt: 'external origin', headings: [], href: 'https://evil.example.test/page.html' },
      { title: 'relative page', excerpt: 'safe path', headings: [], path: 'docs/guide/page.md' },
    ],
  },
  URL,
};

vm.runInNewContext(source.slice(scriptStart, scriptEnd), context);

input.value = '<img';
input.listeners.input.call(input);
assert.equal(results.children.length, 1, 'matching HTML payload renders as one safe result');
const result = results.children[0];
assert.equal(result.tagName, 'a');
assert.equal(result.href, `${origin}/kairos/docs/safe.html`);
assert.equal(result.children[0].textContent, '<img src=x onerror=alert(1)>');
assert.equal(result.children[1].textContent, '<svg onload=alert(2)> excerpt');
assert.equal(result.children[0].children.length, 0, 'payload markup remains plain text');
assert.equal('innerHTML' in results, false, 'renderer does not rely on innerHTML');

input.value = 'javascript';
input.listeners.input.call(input);
assert.equal(results.children[0].className, 'no-results', 'dangerous schemes are rejected');

input.value = 'external';
input.listeners.input.call(input);
assert.equal(results.children[0].className, 'no-results', 'external origins are rejected');

input.value = 'relative';
input.listeners.input.call(input);
assert.equal(results.children[0].href, `${origin}/docs/guide/page.html`);

console.log('Search DOM XSS regressions passed.');
