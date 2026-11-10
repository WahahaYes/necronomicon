// An OpenCode plugin that registers necro's skills in-process, so
// `opencode plugin add opencode-necronomicon` teaches the host the same
// three skills .agents/skills carries. The necro binary stays out of band
// (PyPI/crates.io); the skills document installing it.
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Plugin, Skill } from "@opencode/plugin";

const skillsDir = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "skills");
const names = ["necro", "necro-rules", "necro-adopt"] as const;

function readSkill(name: string): { description: string; content: string } {
  const content = readFileSync(path.join(skillsDir, name, "SKILL.md"), "utf8");
  const frontmatter = content.match(/^---\n([\s\S]*?)\n---\n/);
  const description =
    frontmatter?.[1]
      .split("\n")
      .filter((line) => line.startsWith("description:"))
      .map((line) => line.slice("description:".length).trim())[0] ?? "";
  return { description, content };
}

export default Plugin.define({
  id: "opencode-necronomicon",
  async setup(ctx) {
    await ctx.skill.transform((editor) => {
      for (const name of names) {
        const file = path.join(skillsDir, name, "SKILL.md");
        const { description, content } = readSkill(name);
        editor.add({
          id: `opencode-necronomicon/${name}`,
          name,
          description,
          path: file,
          // Hosts before the `path` field read `location` instead; carry
          // both so old and new hosts load the skill.
          location: file,
          content,
        } as Skill.Info & { location: string });
      }
    });
  },
});
