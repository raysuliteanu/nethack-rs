# README

This is a port of [Nethack](https://github.com/NetHack/NetHack) to Rust, for
"hobby" purposes i.e. my own edification and interest in learning Rust through
doing.

## LICENSE

This project adopts the NetHack General Public License as described in
[LICENSE](LICENSE).

## C NetHack

### Building

tl;dr;

```plain
Prerequisite installations that are needed:
    libncurses-dev
    flex
    bison
    clang or gcc

Recommended steps:

1.  cd sys/unix
2.  sh setup.sh hints/linux
3.  cd ../..
4.  make all; su; make install
```

Above is from `./nethack/sys/unix/README.linux`
