# Function Plus - Generic Type Arguments

## Feature Specification

Function invoke should be async and have type arguments as much as possible inferred from the arguments/return.

### Core Requirements

1. **Generic Function class**
   - `Function<TArgs extends any[], TReturn>` with type parameters for arguments and return type
   - `invoke(...args: TArgs): Promise<TReturn>` with proper typing
   - Types should be inferred from the handler passed to constructor

2. **Generic FunctionHandler type**
   - `FunctionHandler<TArgs, TReturn> = (...args: TArgs) => Promise<TReturn>`
   - Defaults to `any[]` and `any` for backwards compatibility

3. **Generic TBDFunction**
   - `TBDFunction<TArgs, TReturn>` extends `Function<TArgs, TReturn>`
   - Allows defining typed placeholders before implementation is provided

4. **Type-safe overloads**
   - `overload(handler: FunctionHandler<TArgs, TReturn>)` enforces type compatibility
   - Overload handler must match the Function's type signature

### Decisions Taken

- **Default type parameters for backwards compatibility**: Both `Function` and `TBDFunction` default to `any[]` and `any` so existing code without explicit type parameters continues to work.

- **JsonStore is generic over document type**: `JsonStore<TDoc>` allows specifying the document type, which flows through to `storeFunction` and `getFunction` types.

- **Type inference from handler**: When creating a Function with a handler, TypeScript infers the generic types from the handler signature, so explicit type parameters are often unnecessary.

## Implementation Details

### Function class changes

The Function class is now generic with two type parameters:
- `TArgs extends any[]` - tuple type for function arguments
- `TReturn` - return type (wrapped in Promise by invoke)

Type inference works automatically when constructing:
```typescript
// TypeScript infers Function<[string], string>
const helloFn = new Function(arch, 'hello', async (name: string) => {
  return `Hello, ${name}!`;
});

// invoke() now has proper types
const result = await helloFn.invoke('world'); // result: string
```

### TBDFunction for typed placeholders

TBDFunction accepts explicit type parameters for defining API contracts:
```typescript
const storeFn: TBDFunction<[string, Doc], { success: boolean }> = new TBDFunction(scope, 'store');
// invoke() is typed: (collection: string, doc: Doc) => Promise<{ success: boolean }>
```

### JsonStore generic over document type

JsonStore accepts a type parameter for the stored documents:
```typescript
interface Greeting { when: number; name: string; }
const store = new JsonStore<Greeting>(arch, 'greetings');

// store.store() expects Greeting document
// store.get() returns Promise<Greeting[]>
```
