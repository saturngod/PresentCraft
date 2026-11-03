# Skills

Skills that teach AI assistants to work with PresentCraft. Each skill is a folder with a `SKILL.md` and any scripts it uses.

| Skill | What it does |
| --- | --- |
| [create-prcraft](create-prcraft/SKILL.md) | Writes, checks and converts `.prcraft` presentations, and picks or designs themes (colours, fonts, gradients). Includes `scripts/prcraft.py`, which needs only Python 3. |

## Installing a skill

A skill is the whole folder (`SKILL.md` plus `scripts/`). Copy the folder, not just the `SKILL.md`, and keep the folder named `create-prcraft`.

| Tool | Where the folder goes | Scope |
| --- | --- | --- |
| **Claude Code** | `~/.claude/skills/create-prcraft/` | every project |
| | `<project>/.claude/skills/create-prcraft/` | that project (commit it to share it with your team) |
| **Codex** (CLI, IDE and app) | `~/.agents/skills/create-prcraft/` | every project |
| | `<repo>/.agents/skills/create-prcraft/` | that repository |
| **Claude apps** (claude.ai, desktop) | uploaded as a ZIP | your account |

### Claude Code

```bash
mkdir -p ~/.claude/skills
cp -R Skills/create-prcraft ~/.claude/skills/
```

Claude Code picks up new skills while it runs. Type `/create-prcraft` to call the skill directly, or just ask for a presentation.

### Codex

```bash
mkdir -p ~/.agents/skills
cp -R Skills/create-prcraft ~/.agents/skills/
```

Restart Codex if the skill doesn't show up. Older Codex versions read skills from `~/.codex/skills/` instead; copy the folder there if you use one.

### Claude apps (claude.ai and Claude Desktop)

1. Zip the skill folder:
   ```bash
   cd Skills && zip -r create-prcraft.zip create-prcraft -x '*.DS_Store'
   ```
2. Turn on **Create and edit files** in Settings → Capabilities. Skills need it.
3. Go to **Customize → Skills**, click **+**, choose **Create skill → Upload a skill**, and pick `create-prcraft.zip`.

Uploaded skills are private to your account. Claude in the apps can write the `.prcraft` file for you to download. It can't open PresentCraft for you, so open the file yourself.

### Other AI tools

Point the tool at `create-prcraft/SKILL.md` and keep `scripts/` next to it. The helper needs Python 3 and nothing else.

After installing, ask for a presentation, for example "Make a 6-slide PresentCraft deck about our Q3 results in the Bagan theme". The assistant writes a `.prcraft` file that opens in PresentCraft.
