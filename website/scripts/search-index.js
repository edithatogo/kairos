// website/scripts/search-index.js
// Generates a search index from all docs/ markdown files

const fs = require('fs');
const path = require('path');

function walkDir(dir, base = '') {
    const results = [];
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
        const fullPath = path.join(dir, entry.name);
        const relPath = path.join(base, entry.name);
        if (entry.isDirectory() && entry.name !== '.git') {
            results.push(...walkDir(fullPath, relPath));
        } else if (entry.isFile() && entry.name.endsWith('.md')) {
            const content = fs.readFileSync(fullPath, 'utf-8');
            const headings = [];
            for (const line of content.split('\n')) {
                const m = line.match(/^#{1,3}\s+(.+)/);
                if (m) headings.push(m[1]);
            }
            results.push({
                path: relPath.replace(/\\/g, '/'),
                title: headings[0] || entry.name.replace('.md', ''),
                headings: headings,
                excerpt: content.slice(0, 200).replace(/\n/g, ' ')
            });
        }
    }
    return results;
}

const docsDir = path.join(__dirname, '..', '..', 'docs');
const index = walkDir(docsDir, 'docs');

const outputPath = path.join(__dirname, '..', 'build', 'search-index.json');
const json = 'window.searchIndex = ' + JSON.stringify(index) + ';';
fs.writeFileSync(outputPath, json);
console.log(`Search index: ${index.length} entries written to ${outputPath}`);
