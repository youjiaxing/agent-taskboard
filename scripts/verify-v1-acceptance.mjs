import { readdir, readFile } from "node:fs/promises";

const root = new URL("../docs/acceptance/v1/", import.meta.url);
const files = (await readdir(root)).filter((file) => file.endsWith(".md")).sort();
const rows = [];
for (const file of files) {
  const text = await readFile(new URL(file, root), "utf8");
  for (const line of text.split(/\r?\n/)) {
    const match = line.match(/^- \*\*#(\d+)\*\*.*状态：([^。]+)/);
    if (match) rows.push({ file, number: Number(match[1]), status: match[2] });
  }
}
const numbers = rows.map((row) => row.number);
const duplicates = [...new Set(numbers.filter((number, index) => numbers.indexOf(number) !== index))];
const expected = Array.from({ length: 156 }, (_, index) => index + 1);
const missing = expected.filter((number) => !numbers.includes(number));
const unexpected = numbers.filter((number) => !expected.includes(number));
const nonPassing = rows.filter((row) => !row.status.startsWith("通过"));
if (rows.length !== 156 || duplicates.length || missing.length || unexpected.length || nonPassing.length) {
  console.error(JSON.stringify({ rowCount: rows.length, duplicates, missing, unexpected, nonPassing }, null, 2));
  process.exit(1);
}
console.log("v1 acceptance matrix ok: 156 unique stories, all PASS");
