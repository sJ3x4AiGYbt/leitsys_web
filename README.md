# Leitsys - Web

Frontend for [leitsys_api](https://github.com/sJ3x4AiGYbt/leitsys_api), an implementation of the **Ebbinghaus spaced repetition method** for learning. Built with **Rust / Dioxus 0.7**, compiled to WASM.

```
Question created → Step 1 (1d) → Step 2 (3d) → … → Step 7 (90d) → Mastered ✓
                                           → wrong answer → back to Step 1
```

## Features

- Auth: login, signup, email verification, forgot/reset password, session persisted and silently refreshed.
- Categories, questions and steps: create, list, edit, delete, reorder.
- Review: a dedicated loop through due/overdue questions — reveal the answer, grade yourself, watch the question advance or reset.
- Home: a monthly calendar of upcoming reviews (click a day to see what's due), plus late/due-today/this-month/active stats.