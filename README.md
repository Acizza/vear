# vear

Vear is a program to **v**iew, **e**xtract, and mount **ar**chives in a TUI interface that is heavily inspired by [ranger](https://github.com/ranger/ranger).

Currently, only password-less zip archives are supported.

# Usage

Simply launch the program with the path of the archive you want to view, and use the arrow keys to navigate through it.

Multiple entries can be selected by pressing `space`.

# Extracting

You can extract the selected portion of the archive by pressing the `s` key and entering an output path.

# Mounting

You can mount the archive as a read-only [FUSE](https://en.wikipedia.org/wiki/FUSE_%28Linux%29) filesystem by pressing the `l` key and entering a path to mount the archive at.

Please keep in mind that the entire uncompressed size of the archive may be read into memory by other applications.
