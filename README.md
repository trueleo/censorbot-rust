# censorbot-rust
same as the other telegram bot but rewritten in Rust for Blazing Fast Speed and Stability hopefully

- Fully supported and intended to run under Docker
- Very secure with distroless base image
- Completely untested on heroku, but the yml should just work fine.

You will need a postgres database to run this

Set the following env vars
- `BOTNAME` to your bot username
- `BOT_TOKEN` to your bot token
- `DB_NAME`, `DB_HOST`, `DB_USER`, `DB_PASS`, `DB_PORT` are for postgres and are compulsory.

`BOT_TOKEN` and `BOTNAME` are built into the image, and you should not push this image to a public registry for the risk of leaking bot token.

