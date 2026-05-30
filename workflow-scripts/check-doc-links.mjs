import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

if (isMainModule()) {
  const result = validateDocsLinks(path.resolve("docs"));
  if (!result.valid) {
    console.error(
      "Fix the listed documentation links before merging; broken routes block readers and agents from finding the intended source.",
    );
    for (const error of result.errors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }

  console.log("Documentation links resolve.");
}

export function validateDocsLinks(docsRoot) {
  const markdownFiles = listMarkdownFiles(docsRoot);
  const repoRoot = findRepoRoot(docsRoot);
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
    errors.push(
      ...validateVersionedPathReferences({ docsRoot, file, repoRoot, text }),
    );
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

function validateVersionedPathReferences({ docsRoot, file, repoRoot, text }) {
  const errors = [];
  const relativeFrom = path.relative(docsRoot, file);
  if (repoRoot === undefined) {
    return errors;
  }

  for (const reference of localPathReferences(stripFencedCodeBlocks(text))) {
    const repoRelativePath = repoRelativeReference({ file, reference, repoRoot });
    if (repoRelativePath === undefined) {
      continue;
    }

    const check = checkVersioned(repoRoot, repoRelativePath);
    if (!check.versioned) {
      errors.push(
        `${relativeFrom}:${reference.line}: references unversioned path: ${reference.text}`,
      );
    }
  }

  return errors;
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

function localPathReferences(text) {
  const references = [];

  const codePattern = /`([^`\n]+)`/g;
  for (const codeMatch of text.matchAll(codePattern)) {
    const codeText = codeMatch[1];
    if (codeText.includes("](")) {
      continue;
    }
    const codeStart = codeMatch.index + 1;
    for (const candidateMatch of pathCandidates(codeText)) {
      references.push({
        line: lineNumberAt(text, codeStart + candidateMatch.index),
        text: candidateMatch.text,
      });
    }
  }

  return references;
}

function pathCandidates(text) {
  const candidates = [];
  const pattern =
    /(?:\.{1,2}\/)?[A-Za-z0-9._@%-]+(?:\/[A-Za-z0-9._@%-]+)+\/?|(?<![A-Za-z0-9._@%-])[A-Za-z0-9._@%-]+\.(?:class|vsix)\b/g;

  for (const match of text.matchAll(pattern)) {
    const candidate = cleanPathReference(match[0]);
    if (isRepositoryPathReference(candidate)) {
      candidates.push({
        index: match.index,
        text: candidate,
      });
    }
  }

  return candidates;
}

function cleanPathReference(value) {
  return value
    .replace(/#.*/, "")
    .replace(/[),.;:]+$/g, "");
}

function isRepositoryPathReference(value) {
  return (
    value !== "" &&
    !value.startsWith("#") &&
    !isExternalLink(value) &&
    !path.isAbsolute(value) &&
    !isPlaceholderPath(value) &&
    (value.startsWith("./") ||
      value.startsWith("../") ||
      startsWithKnownRepoRoot(value) ||
      hasFileExtension(value))
  );
}

function isPlaceholderPath(value) {
  return value.startsWith("path/to/");
}

function startsWithKnownRepoRoot(value) {
  return /^(?:\.github|crates|docs|editors|prompts|toolchain_cases|workflow-scripts)\//.test(value);
}

function hasFileExtension(value) {
  return /(?:^|\/)[^/]+\.[A-Za-z0-9]+\/?$/.test(value);
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

function repoRelativeReference({ file, reference, repoRoot }) {
  const pathText = reference.text;
  const decoded = decodeUriPath(pathText);
  const relative = pathText.startsWith("./") || pathText.startsWith("../")
    ? repoRelativePath(repoRoot, path.resolve(path.dirname(file), decoded))
    : versionedRelativePath(repoRoot, path.resolve(path.dirname(file), decoded)) ??
      repoRelativePath(repoRoot, path.resolve(repoRoot, decoded));

  if (relative === undefined) {
    return undefined;
  }

  return relative;
}

function versionedRelativePath(repoRoot, absolutePath) {
  const relative = repoRelativePath(repoRoot, absolutePath);
  if (relative === undefined) {
    return undefined;
  }

  return checkVersioned(repoRoot, relative).versioned ? relative : undefined;
}

function repoRelativePath(repoRoot, absolutePath) {
  const relative = path.relative(repoRoot, absolutePath);

  if (relative === "" || relative.startsWith("..") || path.isAbsolute(relative)) {
    return undefined;
  }

  return relative;
}

function checkVersioned(repoRoot, repoRelativePath) {
  const result = spawnSync(
    "git",
    ["ls-files", "--error-unmatch", "--", repoRelativePath],
    { cwd: repoRoot },
  );

  return { versioned: result.status === 0 };
}

function findRepoRoot(start) {
  let current = path.resolve(start);
  while (true) {
    if (fs.existsSync(path.join(current, ".git"))) {
      return current;
    }
    const parent = path.dirname(current);
    if (parent === current) {
      return undefined;
    }
    current = parent;
  }
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
