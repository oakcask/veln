import { readdirSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function githubYamlExtensionViolations(root) {
  const violations = [];
  function visit(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) {
        visit(path);
      } else if (/\.ya?ml$/i.test(entry.name) && !entry.name.endsWith(".yaml")) {
        violations.push(relative(root, path).replaceAll("\\", "/"));
      }
    }
  }
  visit(join(root, ".github/workflows"));
  visit(join(root, ".github/actions"));
  return violations.sort();
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = fileURLToPath(new URL("../", import.meta.url));
  const violations = githubYamlExtensionViolations(root);
  if (violations.length > 0) {
    console.error("Rename these GitHub workflow/action YAML files to use .yaml and update their references; one extension keeps repository checks and path filters consistent:");
    for (const path of violations) console.error(`- ${path}`);
    process.exitCode = 1;
  } else {
    console.log("GitHub workflow/action YAML files use .yaml.");
  }
}
