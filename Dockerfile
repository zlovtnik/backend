FROM oven/bun:1 as base

WORKDIR /app

# Install dependencies
COPY package.json ./
RUN bun install

# Copy source code
COPY . .

# Build the application
RUN bun run build

# Production stage
FROM oven/bun:1-slim

WORKDIR /app

# Install required tools
RUN apt-get update && apt-get install -y \
    netcat \
    && rm -rf /var/lib/apt/lists/*

# Copy built files and dependencies
COPY --from=base /app/package.json ./
COPY --from=base /app/node_modules ./node_modules
COPY --from=base /app/dist ./dist
COPY --from=base /app/prisma ./prisma

# Regenerate Prisma client for the target platform
RUN bunx prisma generate

# Set environment variables
ENV NODE_ENV=production
ENV PORT=3000

# Expose port
EXPOSE 3000

# Run migrations and start the application
CMD ["sh", "-c", "while ! nc -z db 5432; do sleep 1; done && bunx prisma migrate deploy && bun run start"] 