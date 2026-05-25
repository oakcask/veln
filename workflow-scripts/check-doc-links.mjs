import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

if (isMainModule()) {
  const result = validateDocsLinks(path.resolve("docs"));
  if (!result.valid) {
    console.error("Documentation links are broken.");
    for (const error of result.errors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }

  console.log("Documentation links resolve.");
}

export function validateDocsLinks(docsRoot) {
  const markdownFiles = listMarkdownFiles(docsRoot);
  const errors = [];

  for (const file of markdownFiles) {
    const text = fs.readFileSync(file, "utf8");
    const links = localMarkdownLinks(stripMarkdownCode(text));
    for (const link of links) {
      const error = validateLocalLink({ docsRoot, fromFile: file, link });
      if (error !== undefined) {
        errors.push(error);
      }
    }
  }

  return {
    errors,
    valid: errors.length === 0,
  };
}

function validateLocalLink({ docsRoot, fromFile, link }) {
  const [targetPath, rawFragment = ""] = link.target.split("#", 2);
  const targetFile = targetPath === "" ? fromFile : path.resolve(path.dirname(fromFile), decodeUriPath(targetPath));
  const relativeFrom = path.relative(docsRoot, fromFile);

  if (!isWithin(docsRoot, targetFile)) {
    return `${relativeFrom}:${link.line}: link escapes docs: ${link.target}`;
  }

  if (!fs.existsSync(targetFile) || !fs.statSync(targetFile).isFile()) {
    return `${relativeFrom}:${link.line}: missing target: ${link.target}`;
  }

  if (rawFragment !== "") {
    const targetText = fs.readFileSync(targetFile, "utf8");
    const anchors = markdownAnchors(stripFencedCodeBlocks(targetText));
    const fragment = decodeURIComponent(rawFragment);
    if (!anchors.has(fragment)) {
      return `${relativeFrom}:${link.line}: missing anchor: ${link.target}`;
    }
  }

  return undefined;
}

function listMarkdownFiles(root) {
  const files = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...listMarkdownFiles(entryPath));
    } else if (entry.isFile() && entry.name.endsWith(".md")) {
      files.push(entryPath);
    }
  }
  return files.sort();
}

function localMarkdownLinks(text) {
  const links = [];
  const pattern = /(?<!!)\[[^\]\n]+\]\(([^)\n\s]+)(?:\s+"[^"]*")?\)/g;
  for (const match of text.matchAll(pattern)) {
    const target = match[1];
    if (isExternalLink(target)) {
      continue;
    }
    links.push({
      line: lineNumberAt(text, match.index),
      target,
    });
  }
  return links;
}

function markdownAnchors(text) {
  const anchors = new Set();
  const counts = new Map();
  const headingPattern = /^#{1,6}[ \t]+(.+?)[ \t#]*$/gm;

  for (const match of text.matchAll(headingPattern)) {
    const base = slugifyHeading(match[1]);
    const count = counts.get(base) ?? 0;
    counts.set(base, count + 1);
    anchors.add(count === 0 ? base : `${base}-${count}`);
  }

  return anchors;
}

function slugifyHeading(heading) {
  return heading
    .trim()
    .toLowerCase()
    .replaceAll(/`([^`]*)`/g, "$1")
    .replaceAll(/<[^>]+>/g, "")
    .replaceAll(/[^\p{Letter}\p{Number} _-]/gu, "")
    .trim()
    .replaceAll(/[ \t]+/g, "-");
}

function stripFencedCodeBlocks(text) {
  return text.replaceAll(/^```[\s\S]*?^```[ \t]*$/gm, "");
}

function stripMarkdownCode(text) {
  return stripFencedCodeBlocks(text).replaceAll(/`[^`\n]*`/g, "");
}

function isExternalLink(target) {
  return /^[a-z][a-z0-9+.-]*:/i.test(target) || target.startsWith("#");
}

function isWithin(root, target) {
  const relative = path.relative(root, target);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function decodeUriPath(value) {
  return value
    .split("/")
    .map((segment) => decodeURIComponent(segment))
    .join(path.sep);
}

function lineNumberAt(text, offset) {
  let line = 1;
  for (let index = 0; index < offset; index += 1) {
    if (text[index] === "\n") {
      line += 1;
    }
  }
  return line;
}

function isMainModule() {
  return process.argv[1] !== undefined && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
}
