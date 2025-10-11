# Use Node.js 20 Alpine as base image
FROM node:20-alpine AS base

# Install pnpm
RUN npm install -g pnpm@10.18.0

# Set working directory
WORKDIR /app

# Copy package files
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml turbo.json ./
COPY apps/infra-admin/package.json ./apps/infra-admin/
COPY packages/ui/package.json ./packages/ui/

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy source code
COPY . .

# Build the application
RUN pnpm build --filter=infra-admin

# Production stage with nginx
FROM nginx:alpine AS production

# Copy custom nginx configuration
COPY packaging/docker/infra-admin/nginx.conf /etc/nginx/conf.d/default.conf

# Copy built application from build stage
COPY --from=base /app/apps/infra-admin/dist /usr/share/nginx/html/admin

# Expose port
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]
