# rwsh

[![builds.sr.ht status](https://builds.sr.ht/~tudor/rwsh.svg)](https://builds.sr.ht/~tudor/rwsh?)

*(Better name ideas pending)*

This is going to be a UNIX shell based around [Structural Regular Expressions][sre] and the [usam experiment][usam].

[sre]: http://doc.cat-v.org/bell_labs/structural_regexps/
[usam]: https://github.com/tudurom/usam

## Issues:

- [ ] In program output, lines that don't end in a newline are not shown. Probably because of `rustyline`.

## To do:

- [x] Basic command execution with quoted string rules
- [x] Pipes
- [x] Structural regular expressions
    - [x] Addresses
    - [x] Basic commands (`a`, `c`, `i`, `d`, `p`)
    - [x] Loops
- [ ] Shell stuff:
    - [x] Handle signals
    - [x] Load scripts
	- [ ] Redirection
    - [ ] Job control (God have mercy)
- [ ] Variables and variable substitution
    - [x] Strings
    - [x] Assignment
    - [ ] Arrays / Lists
    - [ ] Maps
- [x] Command substitution
- [ ] Control flow structures
    - [x] If-else
    - [ ] While
    - [ ] For
    - [ ] Switch
- [ ] Functions
- [ ] Builtins
    - [x] `cd`
    - [x] `exit`
    - [x] `true` / `false`
    - [x] `eval`
	- [x] `calc`
