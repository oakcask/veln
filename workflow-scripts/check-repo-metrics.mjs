import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const schemaVersion = "veln-repo-metrics-json/v0";
const defaultMaxAnnotations = 50;
const defaultHotspotLimit = 10;
const defaultCycleLimit = 5;

if (isMainModule()) {
  try {
    const report = JSON.parse(fs.readFileSync(0, "utf8"));
    const result = evaluateRepositoryPolicy(report);
    for (const annotation of result.annotations) {
      if (process.env.GITHUB_ACTIONS === "true") {
        console.error(renderGitHubAnnotation(annotation));
      } else {
        console.error(renderConsoleAnnotation(annotation));
      }
    }
    if (result.omittedAnnotationCount > 0) {
      const notice = {
        level: "notice",
        title: "Repository metrics truncated",
        message: `Rerun veln-repo-metrics with --format json to inspect all findings; ${result.omittedAnnotationCount} annotation(s) were omitted to keep CI output usable.`,
      };
      if (process.env.GITHUB_ACTIONS === "true") {
        console.error(renderGitHubAnnotation(notice));
      } else {
        console.error(renderConsoleAnnotation(notice));
      }
    }

    if (process.env.GITHUB_STEP_SUMMARY) {
      fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, result.summary);
    } else {
      console.log(result.summary);
    }
    if (!result.valid) {
      process.exitCode = 1;
    }
  } catch (error) {
    const message = `Regenerate the repository metrics JSON before rerunning this check; CI cannot apply repository policy to an invalid report. ${error.message}`;
    if (process.env.GITHUB_ACTIONS === "true") {
      console.error(renderGitHubAnnotation({
        level: "error",
        title: "Invalid repository metrics report",
        message,
      }));
    }
    console.error(message);
    process.exitCode = 1;
  }
}

export function evaluateRepositoryPolicy(report, options = {}) {
  validateReport(report);
  const maxAnnotations = options.maxAnnotations ?? defaultMaxAnnotations;
  const hotspotLimit = options.hotspotLimit ?? defaultHotspotLimit;
  const cycleLimit = options.cycleLimit ?? defaultCycleLimit;
  const numberedFiles = numberedSplitFiles(report.files);
  const cycles = report.dependency_graph?.cycles ?? [];
  const policyAnnotations = [
    ...numberedFiles.map(numberedFileAnnotation),
    ...cycles.map(dependencyCycleAnnotation),
  ];
  const advisoryAnnotations = report.findings.map(findingAnnotation);
  const allAnnotations = [...policyAnnotations, ...advisoryAnnotations];

  return {
    annotations: allAnnotations.slice(0, maxAnnotations),
    omittedAnnotationCount: Math.max(0, allAnnotations.length - maxAnnotations),
    summary: renderSummary(report, { numberedFiles, hotspotLimit, cycleLimit }),
    valid: numberedFiles.length === 0 && cycles.length === 0,
  };
}

export function numberedSplitFiles(files) {
  const groups = new Map();
  for (const originalPath of files) {
    const normalized = originalPath.replaceAll("\\", "/");
    const directory = path.posix.dirname(normalized);
    const stem = path.posix.basename(normalized, path.posix.extname(normalized));
    const match = stem.match(/^(.*\D)\d+$/u);
    if (!match) {
      continue;
    }
    const key = `${directory}\0${match[1]}`;
    const group = groups.get(key) ?? [];
    group.push(originalPath);
    groups.set(key, group);
  }
  return [...groups.values()]
    .filter((group) => group.length > 1)
    .flat()
    .sort();
}

export function renderSummary(report, options) {
  const graph = report.dependency_graph;
  const hotspots = graph?.hotspots ?? [];
  const cycles = graph?.cycles ?? [];
  const lines = [
    "## Rust Repository Refactor Signals",
    "",
    "Inspect the reported hotspots before broad Rust refactors; keeping responsibilities cohesive limits reviewer scope and cross-module coordination.",
    "",
    `- Rust files analyzed: ${report.summary.rust_file_count}`,
    `- Metric findings: ${report.summary.finding_count}`,
    `- Internal dependency edges: ${graph?.edge_count ?? 0}`,
    `- Dependency cycle groups: ${cycles.length}`,
    `- Numbered split files: ${options.numberedFiles.length}`,
    "",
    "### Highest dependency pressure",
    "",
  ];
  if (hotspots.length === 0) {
    lines.push("No files have both incoming and outgoing internal dependencies in this scan.", "");
  } else {
    lines.push("| File | In | Out | Pressure |", "| --- | ---: | ---: | ---: |");
    for (const hotspot of hotspots.slice(0, options.hotspotLimit)) {
      lines.push(`| \`${escapeMarkdown(hotspot.path)}\` | ${hotspot.incoming} | ${hotspot.outgoing} | ${hotspot.pressure} |`);
    }
    if (hotspots.length > options.hotspotLimit) {
      lines.push("", `${hotspots.length - options.hotspotLimit} more hotspot(s) omitted from this summary.`);
    }
    lines.push("");
  }

  lines.push("### Dependency cycles", "");
  if (cycles.length === 0) {
    lines.push("No internal Rust source cycles were detected.", "");
  } else {
    lines.push("Remove these cycles before merging; cyclic ownership forces changes to coordinate in both directions.", "");
    for (const [index, cycle] of cycles.slice(0, options.cycleLimit).entries()) {
      lines.push(`${index + 1}. ${cycle.map((file) => `\`${escapeMarkdown(file)}\``).join(" -> ")}`);
    }
    if (cycles.length > options.cycleLimit) {
      lines.push("", `${cycles.length - options.cycleLimit} more cycle group(s) omitted from this summary.`);
    }
    lines.push("");
  }

  lines.push("### Numbered split files", "");
  if (options.numberedFiles.length === 0) {
    lines.push("No numbered split file series were detected.", "");
  } else {
    lines.push("Rename these files for the responsibility they own before merging; numbered buckets hide ownership and encourage mechanical splits.", "");
    for (const file of options.numberedFiles) {
      lines.push(`- \`${escapeMarkdown(file)}\``);
    }
    lines.push("");
  }
  return `${lines.join("\n")}\n`;
}

export function renderGitHubAnnotation(annotation) {
  const properties = [];
  if (annotation.file) {
    properties.push(`file=${escapeAnnotationProperty(annotation.file)}`);
  }
  if (annotation.line) {
    properties.push(`line=${annotation.line}`);
  }
  properties.push(`title=${escapeAnnotationProperty(annotation.title)}`);
  return `::${annotation.level} ${properties.join(",")}::${escapeAnnotationMessage(annotation.message)}`;
}

function findingAnnotation(finding) {
  switch (finding.kind) {
    case "abc_complexity":
      return {
        level: "warning",
        file: finding.path,
        line: finding.line,
        title: "High ABC complexity",
        message: `${finding.subject} has ABC ${finding.abc.magnitude.toFixed(1)} (A=${finding.abc.assignments}, B=${finding.abc.branches}, C=${finding.abc.conditionals}); when touching this function, improve cohesion around one concern or decouple distinct concepts so complexity decreases for reviewers.`,
      };
    case "file_line_count":
      return {
        level: "warning",
        file: finding.path,
        line: finding.line,
        title: "Large Rust file",
        message: `${finding.path} has ${finding.lines} lines; when touching this file, check whether its responsibilities share one owner or move distinct concepts behind clearer boundaries so review scope stays understandable.`,
      };
    default:
      throw new Error(`unsupported repository metrics finding kind ${JSON.stringify(finding.kind)}`);
  }
}

function numberedFileAnnotation(file) {
  return {
    level: "error",
    file,
    line: 1,
    title: "Numbered split file",
    message: "Rename this Rust file for the responsibility it owns before merging; numbered bucket files hide ownership and make code-metric refactors mechanical.",
  };
}

function dependencyCycleAnnotation(cycle) {
  const pathText = [...cycle, cycle[0]].join(" -> ");
  return {
    level: "error",
    file: cycle[0],
    line: 1,
    title: "Rust dependency cycle",
    message: `Remove this dependency cycle before merging; cyclic ownership forces changes to coordinate in both directions. Cycle: ${pathText}`,
  };
}

function validateReport(report) {
  if (report?.schema_version !== schemaVersion) {
    throw new Error(`expected schema_version ${JSON.stringify(schemaVersion)}`);
  }
  if (!Array.isArray(report.files) || !Array.isArray(report.findings)) {
    throw new Error("expected files and findings arrays");
  }
  if (!report.summary || !Number.isInteger(report.summary.rust_file_count) || !Number.isInteger(report.summary.finding_count)) {
    throw new Error("expected integer summary counts");
  }
  if (report.dependency_graph !== null) {
    if (!Array.isArray(report.dependency_graph?.hotspots) || !Array.isArray(report.dependency_graph?.cycles)) {
      throw new Error("expected dependency_graph hotspots and cycles arrays");
    }
  }
}

function renderConsoleAnnotation(annotation) {
  const location = annotation.file ? `${annotation.file}:${annotation.line ?? 1}: ` : "";
  return `${annotation.level}: ${location}${annotation.title}: ${annotation.message}`;
}

function escapeAnnotationProperty(value) {
  return value
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A")
    .replaceAll(":", "%3A")
    .replaceAll(",", "%2C");
}

function escapeAnnotationMessage(value) {
  return value.replaceAll("%", "%25").replaceAll("\r", "%0D").replaceAll("\n", "%0A");
}

function escapeMarkdown(value) {
  return value.replaceAll("|", "\\|").replaceAll("`", "\\`");
}

function isMainModule() {
  return process.argv[1] !== undefined && fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);
}
