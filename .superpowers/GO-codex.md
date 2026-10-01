# GO: Codex (Lanes B, C, D, and the FOCUS batches assigned to you)

You run until your tokens run out. Your repos are siblings on the owner's machine: `~/src/bible-atlas` (the atlas, where the queue lives) and `~/src/map-generator`.

Repeat this loop:

1. **Sync.**
   - Fetch both repos.
   - Pull `~/src/bible-atlas-ops` (the atlas's `ops` branch) and read its `QUEUE.md` and the held locks (`.superpowers/LOCKS.md`).
2. **Your own reviews first.**
   - Review every Claude item in `review`: correctness, `docs/PRINCIPLES.md`, and the 14b D.R.Y. and "would a Haskell programmer scoff" pass.
   - Write your findings on the item. Claude lands its work only after your review.
3. **Claim.**
   - Take the first `ready` Codex item whose dependencies are `done`. Priority: Lane B (maps) > FOCUS batches assigned to you > Lane C > Lane D.
   - You may hold **up to 3 items at once**, each in its own worktree, with **at most one Rust-heavy build running** at a time (Claude has the other slot). Analyses and doc-producing items can always run beside a build.
4. **Work the item exactly as written.**
   - Base commit, files, gates, done-when. Test-first (PRINCIPLES 1–4, 15–18).
   - If the item is wrong or ambiguous, write the problem on the item and take another. Never widen an item's scope.
5. **Finish.**
   - Push to `lane/codex/<item-id>` and set the status to `review:<range>`.
   - For map-generator items, also push the lane branch there. Claude lands it.
6. **Questions for the owner** go to OWNER QUESTIONS, one line each, answerable in one line. For errata, batch about 10 rulings per question, most severe first.

**Never:**
- Push to `worktree-bible-atlas-m1` or map-generator's `master`.
- Take a lock you don't need, or hold one past its critical section.
- Edit files outside your item.
- Re-bless a golden view or fixture without a recorded owner approval.

**Before you stop:** write `handoff:` on each claimed item, release your locks, commit and push.
