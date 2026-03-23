# Upstream Sync

## Updating reporters-db and courts-db

```bash
git -C data/reporters-db pull origin main
git -C data/courts-db pull origin main
cargo build  # build.rs re-processes data automatically
cargo test   # verify nothing broke
git add data/reporters-db data/courts-db
git commit -m "chore: update reporters-db and courts-db"
```

## Tracking eyecite changes

```bash
git -C upstream/eyecite pull origin main
# Review changes: git -C upstream/eyecite log --oneline -20
# Port any relevant changes to Rust
```
