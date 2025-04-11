# AI Alibaba Cloud Test

A modern web API built with Elysia, Bun, and PostgreSQL, featuring user-specific database connection management.

## Features

- User-specific database connection limits
- Organization and user management
- Permission-based access control
- RESTful API endpoints
- Type-safe database operations with Prisma

## Prerequisites

- [Bun](https://bun.sh/) (v1.0.0 or later)
- PostgreSQL database
- Node.js (for Prisma CLI)

## Installation

1. Clone the repository
2. Install dependencies:
```bash
bun install
```

3. Set up your environment variables:
```bash
cp .env.example .env
```
Edit `.env` with your database credentials:
```
DATABASE_URL="postgresql://user:password@localhost:5432/database"
```

4. Run database migrations:
```bash
npx prisma migrate dev
```

## Database Connection Management

The system implements user-specific database connection limits:

- Each user has a `maxConnections` field in the database
- Default connection limit is 1
- Connections are automatically managed and released
- System prevents exceeding user's connection limit

### User Connection Limits

- Users with `maxConnections = 1`: One active database connection
- Users with `maxConnections = 10`: Up to 10 active database connections
- Connection limits are enforced per user

## API Endpoints

All endpoints require the `x-user-id` header for user identification.

### Organizations

- `GET /api/organizations`
  - Returns organization statistics
  - Requires: `x-user-id` header

### Users

- `GET /api/users`
  - Returns users by organization
  - Requires: `x-user-id` header

### Permissions

- `GET /api/permissions`
  - Returns user permissions
  - Requires: `x-user-id` header

## Development

Start the development server:
```bash
bun run dev
```

Run tests:
```bash
bun test
```

## Database Schema

### User Model
```prisma
model User {
  id             String        @id @default(uuid())
  email          String        @unique
  name           String
  password       String
  organizationId String
  organization   Organization  @relation(fields: [organizationId], references: [id])
  permissions    Permission[]
  maxConnections Int          @default(1)
  createdAt      DateTime      @default(now())
  updatedAt      DateTime      @updatedAt
}
```

## Error Handling

The API returns appropriate error responses:

- 400: Missing or invalid user ID
- 500: Internal server error
- Connection limit exceeded: Error when user tries to exceed their connection limit

## License

MIT