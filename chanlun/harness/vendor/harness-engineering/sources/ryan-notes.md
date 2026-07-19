# Ryan Lopopolo's Implementation Observations

## Polytoken and Codex `apply_patch`

On July 17, 2026, Ryan Lopopolo supplied this observation based on his knowledge
of Polytoken's implementation:

> Polytoken directly reuses Codex's open-source `apply_patch` implementation for
> OpenAI models.

Polytoken's public documentation describes the model-selected `patch_edit`
surface but does not independently establish implementation reuse.
