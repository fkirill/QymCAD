# The agent

An agent is a program on this machine that works on the open document for you: it draws, edits,
measures, exports - through the same window you work in. It is let in by the agent's server, which
is switched on in the settings and is off unless you switch it on.

## How it works beside you

Everything the agent does appears in the window as you watch: lines land in the sketch, bodies grow,
parts move. While a call of its runs, the window says so with the same card a background rebuild
uses, and the window stays live - you can keep working.

The program is yours first. While you hold a mouse button, type in a field, or have a file chooser
open, the agent waits; its turn comes when your gesture is over.

What the agent makes can be taken back with the usual **Undo**. The steps it creates carry the
agent's name in the undo list, so you can always tell its steps from your own.

## What the agent can do

The agent finds what the program offers by searching the names of its operations - in any language
the program speaks - and calls them by name with their values: every drawing tool, every part
feature, every mate, the view, the measurements, the files.

For whatever has no name of its own, the agent works the window the way you do: it presses a button
by the word the button shows, clicks a place of the sketch or of the space of the model, fills a
field, walks a menu, presses a key, types.

## When something does not work

- **A call was refused** - the refusal says in words what was asked and what the program expected
  (a name it does not carry, a value of the wrong kind, a place where nothing stands to be picked).
  Nothing changes in the document by a refused call.
- **The agent's step came out wrong** - take it back with **Undo**, as with any step of your own.
- **Nothing answers at all** - see whether the server is still switched on in the settings and
  whether the client carries the pairing token; [The agent's server](general/16-agent-server) says
  where both are.
