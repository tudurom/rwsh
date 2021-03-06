# rwsh

[![builds.sr.ht status](https://builds.sr.ht/~tudor/rwsh.svg)](https://builds.sr.ht/~tudor/rwsh?)

*(Better name ideas pending)*

This is going to be a UNIX shell based around [Structural Regular Expressions][sre] and the [usam experiment][usam].

[sre]: http://doc.cat-v.org/bell_labs/structural_regexps/
[usam]: https://github.com/tudurom/usam

## To do:

See `todo/todo.txt`.

- [x] Basic command execution with quoted string rules
- [x] Pipes
- [x] Structural regular expressions
    - [x] Addresses
    - [x] Basic commands (`a`, `c`, `i`, `d`, `p`)
    - [x] Loops
	- [ ] Variable and command substitution (with capture group values)
	    - [ ] In addresses
		- [x] In command parameters
    - [ ] Shell commands in SRE
- [ ] Shell stuff:
    - [x] Load scripts
    - [x] `||`, `&&`, `!`
	- [ ] Redirection
    - [ ] Job control (God have mercy)
        - [ ] Handle signals
- [ ] Variables and variable substitution
    - [x] Strings
    - [x] Assignment
    - [ ] Arrays / Lists
    - [ ] Maps
- [x] Command substitution
- [ ] Control flow structures
    - [x] If-else
    - [x] While
    - [ ] For
	    - [x] String globbing
    - [x] Switch
	- [x] Matches (awk-like pattern list)
- [ ] Functions
- [ ] Builtins
    - [x] `cd`
    - [x] `exit`
    - [x] `true` / `false`
    - [x] `eval`
	- [x] `calc`
	- ...
