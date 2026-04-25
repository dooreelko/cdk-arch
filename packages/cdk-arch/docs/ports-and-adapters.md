# Ports & Adapters Integration in cdk-arch

## Overview

cdk-arch can be extended to support traditional hexagonal (ports and adapters) architecture, providing a clear separation between:

1. **Inner Hexagon (Domain):** Core entities, use cases, and business logic
2. **Port Interfaces:** Clear contracts for interaction
3. **Adapter Implementations:** External components that implement ports

## Architecture

```
┌─────────────────────────────────────────────┐
│           Outer Hexagon (Adapters)          │
├─────────────────────────────────────────────┤
│  • UserService (primary port impl)          │
│  • InMemoryUserRepository (secondary impl)  │
│  • HTTP Routes → Port methods               │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│         Boundary: Port Interfaces           │
├─────────────────────────────────────────────┤
│  • UserPort (primary)                       │
│  • UserPersistencePort (secondary)          │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│         Inner Hexagon (Use Cases)           │
├─────────────────────────────────────────────┤
│  • UserUseCase                              │
│  • Domain Entities                          │
│  • Business Logic                           │
└─────────────────────────────────────────────┘
```

## Components

### 1. Ports - Contract Definitions

Define your interfaces at `src/ports/`:

```typescript
// src/ports/user-port.ts

export interface User {
  id: string;
  name: string;
  email: string;
}

// Primary port - what core provides
export interface UserPort {
  listUsers(): Promise<User[]>;
  getUserById(id: string): Promise<User>;
  createUser(dto: CreateUserDTO): Promise<User>;
}

// Secondary port - what infrastructure needs
export interface UserPersistencePort {
  saveUser(user: User): void;
  findUserById(id: string): User | undefined;
}
```

### 2. Use Cases - Domain Logic

Use cases live in `src/use-cases/` and depend on secondary ports:

```typescript
// src/use-cases/user-use-case.ts

export class UserUseCase implements UserPersistencePort {
  private users = new Map<string, User>();

  async listUsers(): Promise<User[]> {
    return Array.from(this.users.values());
  }

  async getUserById(id: string): Promise<User | undefined> {
    return this.users.get(id);
  }

  async createUser(dto: { name: string; email: string }): Promise<User> {
    const newUser: User = {
      id: crypto.randomUUID(),
      ...dto,
    };
    this.saveUser(newUser); // Uses persistence port internally
    return newUser;
  }

  // Secondary port implementation
  saveUser(user: User): void {
    this.users.set(user.id, user);
  }

  findUserById(id: string): User | undefined {
    return this.users.get(id);
  }
}
```

### 3. Adapters - External Implementations

Implement ports in `src/adapters/`:

```typescript
// src/adapters/user-service.ts

export class UserService implements UserPort {
  constructor(private readonly useCase: UserUseCase) {}

  async listUsers(): Promise<User[]> {
    return this.useCase.listUsers();
  }

  async getUserById(id: string): Promise<User | undefined> {
    return this.useCase.getUserById(id);
  }

  async createUser(dto: { name: string; email: string }): Promise<User> {
    return this.useCase.createUser(dto);
  }
}
```

```typescript
// src/adapters/http-user-adapter.ts

export class HttpUserAdapter implements UserPort {
  constructor(private readonly baseUrl: string) {}

  async listUsers(): Promise<User[]> {
    const res = await fetch(`${this.baseUrl}/users`);
    return res.json();
  }

  async getUserById(id: string): Promise<User> {
    const res = await fetch(`${this.baseUrl}/users/${id}`);
    if (!res.ok) throw new Error('Not found');
    return res.json();
  }

  async createUser(dto: CreateUserDTO): Promise<User> {
    const res = await fetch(`${this.baseUrl}/users`, {
      method: 'POST',
      body: JSON.stringify(dto),
    });
    return res.json();
  }
}
```

### 4. API Integration

Use API containers to expose port methods as routes:

```typescript
// Composite example
export class UserComposite {
  static createUserApi(arch: Architecture, service: UserService): ApiContainer {
    return new ApiContainer<ApiRoutes>(arch, 'user-api', {
      list: {
        path: 'GET /users',
        handler: new Function<[], User[]>(
          arch,
          'list-users',
          async () => service.listUsers()
        ),
      },
      get: {
        path: 'GET /users/{id}',
        handler: new Function<[string], User>(
          arch,
          'get-user',
          async (id) => service.getUserById(id)
        ),
      },
      create: {
        path: 'POST /users',
        handler: new Function<[{ name: string; email: string }], User>(
          arch,
          'create-user',
          async (dto) => service.createUser(dto)
        ),
      },
    });
  }
}
```

## Usage Example

```typescript
import { Architecture } from '@arinoto/cdk-arch';
import { ArchitectureBinding } from '@arinoto/cdk-arch/binding';
import { UserService } from '@arinoto/cdk-arch/adapters/user-service';
import { UserComposite } from '@arinoto/cdk-arch/adapters/user-composite';

// Create architecture and bindings
const arch = new Architecture();
const binding = new ArchitectureBinding();

// 1. Create use case (inner hexagon)
const useCase = new UserUseCase();

// 2. Create service adapter (outer hexagon)
const userService = new UserService(useCase);

// 3. Create API container
const api = UserComposite.createUserApi(arch, userService);

// 4. Bind to endpoint
binding.bind(api, { baseUrl: 'http://localhost:3000' });

// The API is now accessible via HTTP at http://localhost:3000
```

## Benefits

### 1. Testability
```typescript
// Inject mock repository for testing
const mockRepo = {
  saveUser: vi.fn(),
  findUserById: vi.fn(() => mockUser),
};

const useCase = new UserUseCase(mockRepo);

// Test without actual data storage
expect(useCase.createUser(dto)).toEqual(newUser);
```

### 2. Swap Implementations
```typescript
// Use in-memory persistence for development
const useCase = new UserUseCase(new InMemoryUserRepository());

// Switch to database for production
const useCase = new UserUseCase(new PostgresUserRepository());

// No code changes needed!
```

### 3. Clear Boundaries
- **Entities** - Core business objects (User)
- **Use Cases** - Business operations (UserUseCase)
- **Ports** - Clear contracts on the boundary
- **Adapters** - External implementations at the edge

## Integration with cdk-arch

cdk-arch integrates ports and adapters by:

1. **Using Functions as port implementations** - Each adapter implements a port
2. **ApiContainer as the router** - Routes API endpoints to adapter methods
3. **ArchitectureBinding for runtime binding** - Swaps implementations dynamically

This combines CDK's declarative infrastructure with hexagonal architecture's decoupled design.