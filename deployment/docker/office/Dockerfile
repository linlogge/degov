FROM node:22-slim AS builder

WORKDIR /app

RUN npm install -g pnpm@10.18.0

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/office/package.json ./apps/office/
COPY packages/ ./packages/
COPY patches/ ./patches/

RUN pnpm install --frozen-lockfile

COPY apps/office/ ./apps/office/

WORKDIR /app/apps/office
RUN pnpm build

FROM nginx:alpine

COPY --from=builder /app/apps/office/dist /usr/share/nginx/html

COPY deployment/docker/office/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

