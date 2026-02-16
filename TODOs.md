# TODOs

- support C nethack command line options and environment variables, e.g. to find
  where save files are located, bones files are located, etc. so they can be
  read, just as the C nethack would read them on startup. For example, see
  `nethack/sys/unix/unixmain.c`
  The C nethack build seems to hard-code defaults; we should perhaps do something
  similar but via a `build.rs` using the same defaults depending on platform
  (though let's support only Linux for now).
- port the C Nethack code in `nethack/utils`, specifically the level compiler
  (`lev_comp`), the dungeon compiler (`dgn_comp`)
- investigate the C Nethack save/restore capabilities include checkpointing and
  the use of the `recover` command in nethack/util
- support a build package step that creates a tarball or maybe a zip file or
  similar archive (e.g. 7zip) that includes the compiled game binary, any
  utilities (e.g. the `recover` command) and associated game assets, including
  compiled levels and such. Can this be done via `cargo`, perhaps via some
  existing crates, or maybe using something like `just` or just a Bash script.
  Since this port is a drop in replacement, from the user's perspective everything
  should be identical, so we should also use and ship the Nethack help docs, man
  pages, etc. both in-game as well as via the distribution tarball; see the
  `Makefile` in nethack/doc
- see if the following GitHub issue exists in the port:
  <https://github.com/NetHack/NetHack/issues/1021>
  It references this commit as a fix:
  <https://github.com/NetHack/NetHack/commit/e43ec0cef1cc7c4e639c7de7e1d9ff26c6c8c629>
