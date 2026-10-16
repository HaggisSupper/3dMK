# Copilot Development Instructions Setup

## What happens

1. A reviewed change to .github/DEV_INSTRUCTIONS.md lands on main.
2. The workflow computes the committed content diff from the previous commit.
3. It creates one GitHub issue containing that diff.
4. It assigns the issue to copilot-swe-agent[bot] with main as the base branch.
5. Copilot creates a draft pull request.
6. The supervisor reviews the pull request and updates DEV_INSTRUCTIONS.md with the next approved task.

The workflow watches committed content, not filesystem modified timestamps. GitHub does not reliably preserve local filesystem timestamps in the repository.

## Required settings

Enable Copilot cloud agent for HaggisSupper/3dMK.

Create a fine-grained user token with access to this repository and:

- Metadata: read
- Actions: read and write
- Contents: read and write
- Issues: read and write
- Pull requests: read and write

Store the token as the repository Actions secret:

COPILOT_AGENT_TOKEN

The GitHub documentation states that the Copilot agent task and issue-assignment APIs require a user-to-server token. The normal workflow GITHUB_TOKEN is not sufficient for this assignment API.

## First activation

After the pull request containing this automation is merged:

1. Add COPILOT_AGENT_TOKEN under Settings → Secrets and variables → Actions.
2. Confirm Copilot cloud agent is available in the repository.
3. Edit .github/DEV_INSTRUCTIONS.md with the next approved task and merge the change to main.
4. Verify that the workflow creates and assigns the issue.
5. Review Copilot’s draft pull request; do not merge it automatically.

## Safety

- Copilot never receives permission to edit main directly.
- The trigger runs only on main and only when DEV_INSTRUCTIONS.md changes.
- Copilot changes on its branch do not recursively trigger the workflow.
- The task file is not editable by Copilot.
- The reviewer remains responsible for tests, architecture, security, memory behavior, and merge approval.
