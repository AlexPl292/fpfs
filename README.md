# Fpfs

FUSE filesystem based on telegram.

## Setup:

- Create a new telegram application: https://core.telegram.org/api/obtaining_api_id
- Set up env variables:
    - **TG_ID** - Get this from the previous step
    - **TG_HASH** - Get this from the previous step
    - **TG_USER_ID** - Your user ID
    - **TG_ACCESS_HASH** - Telegram access hash
- Obtain a session file:  
    Now the tricky part. At the moment this library doesn't support user authentication.
    So please:
    - Load grammers project: https://github.com/Lonami/grammers
    - Start a `dialogs.rs` example: https://github.com/Lonami/grammers/blob/master/lib/grammers-client/examples/dialogs.rs
      
      You'll get a `dialogs.session` file in the root of the project.
    - Copy `dialogs.session` to the root of fpfs project and rename it to `fpfs.session`
- Start:
  - `main.rs` and pass the mount path as a last argument, or
  - integration tests: `tests/integration_tests.rs`
