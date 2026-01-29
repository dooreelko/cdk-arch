# general

the packages in the project are managed using npm workspaces.
a task cannot be completed before 
- `npm run build` from root succeeds
- `npm run e2e` from packages/example/local-docker succeeds
- keep original task description in moth files, append specification-relevant parts

# for typescript

- use functional style when working with arrays, e.g. instead of for-loops use .map, etc
- prefer passing a function instead of using an OO overload
- use `npm run ...` for building, testing, etc.