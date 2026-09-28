import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('./build.js', import.meta.url), 'utf8');
const start = source.indexOf('<script>\n(function() {', source.indexOf('function renderPage'));
assert.notEqual(start, -1, 'search client script exists in generated page template');
const scriptStart = source.indexOf('\n', start) + 1;
const scriptEnd = source.indexOf('</script>', scriptStart);
assert.notEqual(scriptEnd, -1, 'search client script is closed');
const searchClient = source.slice(scriptStart, scriptEnd);

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
  appendChild(child) { this.children.push(child); return child; }
  replaceChildren(...children) { this.children = children; this.childNodes = this.children; }
  addEventListener(name, callback) { this.listeners[name] = callback; }
}

const input = new Element('input');
const results = new Element('div');
const documentListeners = {};
const document = {
  createElement: (tag) => new Element(tag),
  getElementById: (id) => id === 'search' ? input : id === 'search-results' ? results : null,
  addEventListener: (name, callback) => { documentListeners[name] = callback; },
};
const context = {
  document,
  window: {
    location: { href: 'https://docs.example.test/kairos/docs/search.html' },
    searchIndex: [
      { title: '<img src=x onerror=alert(1)>', excerpt: '<svg onload=alert(2)> excerpt', headings: [], href: '/safe.html' },
      { title: 'javascript link', excerpt: 'scheme attack', headings: [], href: 'javascript:alert(3)' },
      { title: 'relative page', excerpt: 'safe path', headings: [], path: 'docs/guide/page.md' },
    ],
  },
  URL,
};
vm.runInNewContext(searchClient, context);

input.value = '<img';
input.listeners.input.call(input);
assert.equal(results.children.length, 1, 'only the safe matching result is rendered');
const result = results.children[0];
assert.equal(result.tagName, 'a');
assert.equal(result.href, 'https://docs.example.test/safe.html');
assert.equal(result.children[0].textContent, '<img src=x onerror=alert(1)>');
assert.equal(result.children[1].textContent, '<svg onload=alert(2)> excerpt');
assert.equal(result.children[0].children.length, 0, 'payload markup remains plain text');
assert.equal('innerHTML' in results, false, 'renderer does not rely on innerHTML');

input.value = 'javascript';
input.listeners.input.call(input);
assert.equal(results.children.length, 1);
assert.equal(results.children[0].className, 'no-results', 'dangerous schemes are rejected');

input.value = 'relative';
input.listeners.input.call(input);
assert.equal(results.children[0].href, 'https://docs.example.test/docs/guide/page.html');

console.log('Search DOM XSS regressions passed.');
