# DeGov - Dezentralized government administration

## Getting Started

As this platform is heavily based on microservices, it is recommended to use Skaffold to build and deploy the services to a Kubernetes cluster. It makes use of caching where possible to speed up the build process to allow quick local development iteration.

### Prerequisites

- [Skaffold](https://skaffold.dev/docs/install/) installed
- A Kubernetes cluster running and accessible via `kubectl` (e.g. [Minikube](https://minikube.sigs.k8s.io/docs/start/) or [Colima](https://github.com/abiosoft/colima))
- Docker installed and running (for local builds)
- `kubectl` configured to connect to your cluster

### Quick Start

#### Deploy with Skaffold

To build and deploy all degov services to your Kubernetes cluster:

```bash
skaffold run
```

This will:
1. Build Docker images for three services:
   - `degov/chancelor` - Core governance service
   - `degov/kube-operator` - Kubernetes operator
   - `degov/frontdoor` - HTTP gateway service
2. Deploy them to your cluster using Helm

#### Development Mode

For active development with automatic rebuilds and redeploys on code changes:

```bash
skaffold dev
```

This runs Skaffold in watch mode, automatically:
- Rebuilding images when source code changes
- Redeploying updated services to your cluster
- Streaming logs from all services

Press `Ctrl+C` to stop the development session and clean up resources.

#### Verify Deployment

Check that all pods are running:

```bash
kubectl get pods -l app.kubernetes.io/instance=degov
```

View logs from a specific service:

```bash
# Chancelor logs
kubectl logs -l app.kubernetes.io/component=chancelor

# Kube Operator logs
kubectl logs -l app.kubernetes.io/component=kube-operator

# Frontdoor logs
kubectl logs -l app.kubernetes.io/component=frontdoor
```

#### Access Services

The frontdoor service includes ingress support. By default, it is available at `http://frontdoor.localhost`.

#### Clean Up

To remove all deployed resources:

```bash
skaffold delete
```

Or if you used `skaffold dev`, simply stop it with `Ctrl+C` and it will clean up automatically.
