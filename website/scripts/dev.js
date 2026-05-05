const fs = require("fs");
const http = require("http");
const path = require("path");

const { build } = require("./build");

const root = path.resolve(__dirname, "..");
const outFile = path.join(root, "build", "index.html");
const sourceFile = path.join(root, "src", "index.md");
const port = Number(process.env.PORT || 3000);

let timer = null;

function rebuild() {
  build();
  process.stdout.write(`Rebuilt ${outFile}\n`);
}

function scheduleRebuild() {
  if (timer) {
    clearTimeout(timer);
  }
  timer = setTimeout(() => {
    rebuild();
  }, 150);
}

rebuild();

const server = http.createServer((req, res) => {
  if (req.url !== "/" && req.url !== "/index.html") {
    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain; charset=utf-8");
    res.end("Not found");
    return;
  }

  res.statusCode = 200;
  res.setHeader("Content-Type", "text/html; charset=utf-8");
  res.end(fs.readFileSync(outFile, "utf8"));
});

server.listen(port, () => {
  process.stdout.write(`Docs dev server running at http://localhost:${port}\n`);
});

fs.watch(sourceFile, { persistent: true }, scheduleRebuild);
