# Locks

A lock is a branch on `origin` named `lock/<name>`. Git refuses to create a branch that already exists, so taking a lock is atomic for both agents, whether they share a machine or not.

The lock names and what each one covers are listed in `AGENTS.md`.

## Take
```bash
name=heavy; item=CX-M2; agent=codex
tree=$(git hash-object -t tree /dev/null)
c=$(git commit-tree "$tree" -m "LOCK $name held by $agent for $item since $(date -Is)")
git push --force-with-lease="refs/heads/lock/$name:" origin "$c:refs/heads/lock/$name"
```
The empty value after the `:` means "only if it doesn't exist yet". A rejected push means someone else holds the lock: do other work and try again later. Never retry in a tight loop.

## Show
```bash
git fetch origin "+refs/heads/lock/*:refs/remotes/origin/lock/*" --prune
git for-each-ref refs/remotes/origin/lock --format='%(refname:short)  %(contents:subject)'
```

## Release
Release the moment the critical section ends:
```bash
git push origin --delete "lock/$name"
```

## Stale locks
A lock held for more than 3 hours with no activity from its holder is stale. Write it under OWNER QUESTIONS in the queue. Only the holder or the owner may delete someone else's lock.
