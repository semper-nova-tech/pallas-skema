# pallas-skema

AI and human friendly diagram protocol.

- [Product brief and technical design](pallas-skema.md)
- [Brainstorming and delivery strategy](brainstorming.md)

Repository: [https://github.com/semper-nova-tech/pallas-skema](https://github.com/semper-nova-tech/pallas-skema)

JSON > Diagram > Editor > SVG. Bridging the design and comprehension gap

## Local serve

From `skema-editor`:

```bash
pnpm start
```

That runs `ng serve`. Open http://localhost:4200/.

Save and validate also need the bridge, in a second terminal from `skema-editor`:

```bash
pnpm start:bridge
```
