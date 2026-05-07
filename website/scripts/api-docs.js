const fs = require('fs');
const path = require('path');

const CRATES_DIR = path.join(__dirname, '..', '..', 'crates');
const BUILD_DIR = path.join(__dirname, '..', 'build', 'api');
const DOCS_DIR = path.join(__dirname, '..', '..', 'docs');

function extractPublicApi(filePath) {
    const content = fs.readFileSync(filePath, 'utf-8');
    const lines = content.split('\n');
    const items = [];
    let inImpl = false;
    let currentFn = null;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];

        const fnMatch = line.match(/^\s*pub\s+fn\s+(\w+)\s*\(/);
        if (fnMatch) {
            items.push({
                kind: 'function',
                name: fnMatch[1],
                line: i + 1,
                file: path.relative(CRATES_DIR, filePath),
                signature: line.trim()
            });
        }

        const structMatch = line.match(/^\s*pub\s+struct\s+(\w+)/);
        if (structMatch) {
            items.push({
                kind: 'struct',
                name: structMatch[1],
                line: i + 1,
                file: path.relative(CRATES_DIR, filePath)
            });
        }

        const enumMatch = line.match(/^\s*pub\s+enum\s+(\w+)/);
        if (enumMatch) {
            items.push({
                kind: 'enum',
                name: enumMatch[1],
                line: i + 1,
                file: path.relative(CRATES_DIR, filePath)
            });
        }

        const traitMatch = line.match(/^\s*pub\s+trait\s+(\w+)/);
        if (traitMatch) {
            items.push({
                kind: 'trait',
                name: traitMatch[1],
                line: i + 1,
                file: path.relative(CRATES_DIR, filePath)
            });
        }
    }
    return items;
}

function generateApiDocs() {
    const allItems = [];

    function walkDir(dir) {
        const entries = fs.readdirSync(dir, { withFileTypes: true });
        for (const entry of entries) {
            if (entry.name === '.git') continue;
            const fullPath = path.join(dir, entry.name);
            if (entry.isDirectory()) {
                walkDir(fullPath);
            } else if (entry.isFile() && entry.name.endsWith('.rs') && !entry.name.endsWith('_test.rs')) {
                const items = extractPublicApi(fullPath);
                allItems.push(...items);
            }
        }
    }

    const crates = fs.readdirSync(CRATES_DIR, { withFileTypes: true })
        .filter(e => e.isDirectory())
        .map(e => e.name);

    for (const crateName of crates) {
        const crateDir = path.join(CRATES_DIR, crateName, 'src');
        if (fs.existsSync(crateDir)) {
            const crateItems = [];
            const files = fs.readdirSync(crateDir).filter(f => f.endsWith('.rs'));
            for (const file of files) {
                const filePath = path.join(crateDir, file);
                const items = extractPublicApi(filePath);
                crateItems.push(...items.map(i => ({ ...i, crate: crateName })));
            }
            allItems.push(...crateItems);
        }
    }

    const grouped = {};
    for (const item of allItems) {
        if (!grouped[item.crate]) grouped[item.crate] = [];
        grouped[item.crate].push(item);
    }

    fs.mkdirSync(BUILD_DIR, { recursive: true });

    for (const [crateName, items] of Object.entries(grouped)) {
        const md = [
            `# ${crateName}`,
            '',
            `## Public API surface`,
            '',
            `Total public items: ${items.length}`,
            '',
            '| Kind | Name | Source |',
            '|------|------|--------|',
        ];

        items.sort((a, b) => a.kind.localeCompare(b.kind) || a.name.localeCompare(b.name));
        for (const item of items) {
            const source = item.signature ? `\`${item.signature}\`` : item.file;
            md.push(`| ${item.kind} | \`${item.name}\` | ${source} |`);
        }

        md.push('', `*Generated from ${crateName} source code*`);
        fs.writeFileSync(path.join(BUILD_DIR, `${crateName}.md`), md.join('\n'));
    }

    const indexJson = JSON.stringify(grouped, null, 2);
    fs.mkdirSync(path.join(__dirname, '..', 'build', 'api'), { recursive: true });
    fs.writeFileSync(path.join(__dirname, '..', 'build', 'api', 'index.json'), indexJson);

    console.log(`API docs: ${Object.keys(grouped).length} crates indexed, ${allItems.length} public items`);
}

if (require.main === module) {
    generateApiDocs();
}

module.exports = { generateApiDocs, extractPublicApi };
