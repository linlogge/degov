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
COPY packages/sdk/package.json ./packages/sdk/

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy source code
COPY . .

ARG API_URL

# Build the application
RUN pnpm build --filter=infra-admin

# Production stage with nginx
FROM nginx:alpine AS production

# Copy custom nginx configuration template
COPY packaging/docker/infra-admin/nginx.conf /etc/nginx/templates/default.conf.template

# Copy built application from build stage
COPY --from=base /app/apps/infra-admin/dist /usr/share/nginx/html

# Expose port
EXPOSE 80

# Environment variable that will be substituted
ENV API_URL=http://localhost:3030/api

# Start nginx (nginx:alpine automatically processes /etc/nginx/templates/*.template with envsubst)
CMD ["nginx", "-g", "daemon off;"]
