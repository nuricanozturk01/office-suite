# Static web app. The `dist/` folder must be built on the host first, with the API origin baked in:
#   VITE_API_URL=https://myoffice-api.nuricanozturk.com pnpm --filter @office/web build
# Build from apps/web:
#   docker build -t repo.repsy.io/nuricanozturk/myoffice-docker/web:latest apps/web
FROM nginxinc/nginx-unprivileged:1.27-alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY dist /usr/share/nginx/html
EXPOSE 8080
