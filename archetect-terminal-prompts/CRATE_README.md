# Archetect Terminal Prompts

This crate is a vendoring of the excellent [inquire](https://github.com/mikaelmello/inquire) terminal prompting library.

This vendored version of inquire __is inferior__ to the actual inquire project, and is stictly intened for exclusive use
within Archetect.

## Vendoring Justification

Archetect currently supports an additional prompt type not included in inquire's out-of-the-box prompts: List - a prompt
that allows a user to provide multiple items as an array.  It is also intended to support additional enhancements to 
inquire that are not within the spirit of inquire's functionality, particularly around the 'help' guidances.  

In addition, the primitives within inquire that would be useful for defining new prompts are not currently publicly 
available as part of inquires API, and therefore requires a fork of the project to add your own.

There may be enhancements within this crate that might be useful to pull into inquire at some point, but would likely
not be incorporated without requiring changes to the ones implemented here.