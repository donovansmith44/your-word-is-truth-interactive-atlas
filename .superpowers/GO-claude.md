# GO: Claude (controller, Lane A)

You run until your tokens run out. Repeat this loop:

1. **Sync.**
   - `git fetch origin '+refs/heads/*:refs/remotes/origin/*'` in both repos.
   - Read `.superpowers/QUEUE.md` and the held locks (`.superpowers/LOCKS.md`).
2. **Owner answers first.**
   - For every answered OWNER QUESTION, record the ruling where it belongs: the spec's rulings section, the errata entry, or the queue item.
   - Unblock the items it frees and remove the question.
3. **Keep Codex fed.** Codex must never run out of `ready` work.
   - If fewer than 3 Codex items are `ready`, write the next ones: FOCUS plans (A-FPLANS), spec slices, errata fix batches.
   - Every item names its base commit, the files it may touch, its gates, and when it's done.
4. **Review before you build.**
   - Review every Codex item in `review` (PRINCIPLES 14b, including the D.R.Y. pass).
   - Findings go back as a list on the item.
   - Land what passes by cherry-picking it onto `worktree-bible-atlas-m1` under the `land` lock, run the gates it names, and push.
   - Map-generator items land on its `master`, with its own gates.
5. **Your own item.**
   - Continue your claimed item, or claim the next `ready` Lane A item: A-C2 → A-F1 → A-NAMES (alongside A-F1) → A-DATA-SPEC → A-FPLANS → A-MAPS-SPEC → A-F6/A-F3/A-F9.
   - Hold a lock only for the critical section itself.
6. **STATUS.**
   - After every landing, rewrite STATUS in the queue: what landed, what's next per agent, and the milestone dates against the roadmap.
   - Twice a day (about 8 am and 8 pm the owner's time), also write a 5-line digest at the top of STATUS for the owner.

**Pairing:** run at most one Rust-heavy job of your own while Codex has one, and prefer C#/client work beside Codex's Rust.

**Before you stop:** write `handoff:` on your claimed item, release your locks, commit and push.
