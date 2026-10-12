# The agent's server

The agent's server is what lets a program on this machine - an AI agent - work on the open document
through the window: it serves one agent for the document, and everything the agent does appears in
the window, can be watched as it happens, and can be undone. What the agent is and how it works
beside you is told of in [The agent](general/15-agent); this article is about the server itself -
where it is switched on and how a client is let in.

## Where it is switched

The server lives in the settings, in the **Agent** section. It is off unless you switch it on, and
switching it on or off takes effect at once, without restarting the program.

While it is on, it listens on this machine only - no other machine can reach it - at the port named
in the section. If another program has taken the port, name another one; the change takes effect at
once.

## The pairing token

A client is let in only when it carries the **pairing token** shown beside the switch. Copy it into
the client's settings with the **Copy** button. A request without the token, or with a wrong one, is
refused - and nothing in the program changes.

The token is made when the server is first switched on.

## When it does not answer

- **Nothing answers at all** - see whether the server is still switched on, and whether the client
  carries the token as it was shown, without spaces around it.
- **The port is taken by another program** - name another port in the section.
- **The agent's step came out wrong** - take it back with **Undo**; the agent's steps carry its name
  in the undo list, so you can tell them from your own.
