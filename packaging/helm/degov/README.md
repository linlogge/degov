# degov Helm Chart

This Helm chart deploys the degov platform on Kubernetes, including:
- **chancelor**: Core governance service
- **kube-operator**: Kubernetes operator for managing degov resources
- **frontdoor**: HTTP gateway service

## Prerequisites

- Kubernetes 1.19+
- Helm 3.0+
- Container images for degov services

## Installation

### Basic Installation

```bash
helm install degov ./packaging/helm/degov
```

### Installation with Custom Values

```bash
helm install degov ./packaging/helm/degov -f my-values.yaml
```

### Upgrade

```bash
helm upgrade degov ./packaging/helm/degov
```

### Uninstall

```bash
helm uninstall degov
```

## Configuration

The following table lists the configurable parameters and their default values:

### Global Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `global.imageRegistry` | Global Docker image registry | `""` |
| `global.imagePullSecrets` | Global Docker registry secret names | `[]` |
| `global.storageClass` | Global storage class for dynamic provisioning | `""` |

### Chancelor Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `chancelor.enabled` | Enable chancelor deployment | `true` |
| `chancelor.image.repository` | Chancelor image repository | `degov/chancelor` |
| `chancelor.image.tag` | Chancelor image tag | `0.1.0` |
| `chancelor.image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `chancelor.replicaCount` | Number of replicas | `1` |
| `chancelor.resources` | Resource requests and limits | See values.yaml |

### Kube Operator Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `kubeOperator.enabled` | Enable kube-operator deployment | `true` |
| `kubeOperator.image.repository` | Kube-operator image repository | `degov/kube-operator` |
| `kubeOperator.image.tag` | Kube-operator image tag | `0.1.0` |
| `kubeOperator.image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `kubeOperator.replicaCount` | Number of replicas | `1` |
| `kubeOperator.serviceAccount.create` | Create service account | `true` |
| `kubeOperator.rbac.create` | Create RBAC resources | `true` |
| `kubeOperator.resources` | Resource requests and limits | See values.yaml |

### Frontdoor Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `frontdoor.enabled` | Enable frontdoor deployment | `true` |
| `frontdoor.image.repository` | Frontdoor image repository | `degov/frontdoor` |
| `frontdoor.image.tag` | Frontdoor image tag | `0.1.0` |
| `frontdoor.image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `frontdoor.replicaCount` | Number of replicas | `1` |
| `frontdoor.service.type` | Service type | `ClusterIP` |
| `frontdoor.service.port` | Service port | `8080` |
| `frontdoor.listenAddress` | Listen address | `0.0.0.0:8080` |
| `frontdoor.resources` | Resource requests and limits | See values.yaml |

### Pod Disruption Budget

| Parameter | Description | Default |
|-----------|-------------|---------|
| `podDisruptionBudget.enabled` | Enable PodDisruptionBudget | `false` |
| `podDisruptionBudget.minAvailable` | Minimum available pods | `1` |

### Service Monitor (Prometheus)

| Parameter | Description | Default |
|-----------|-------------|---------|
| `serviceMonitor.enabled` | Enable ServiceMonitor | `false` |
| `serviceMonitor.interval` | Scrape interval | `30s` |
| `serviceMonitor.scrapeTimeout` | Scrape timeout | `10s` |

## Examples

### Example: Custom Image Registry

```yaml
global:
  imageRegistry: "registry.example.com"
  imagePullSecrets:
    - name: regcred

chancelor:
  image:
    repository: degov/chancelor
    tag: "latest"
```

### Example: Production Configuration

```yaml
chancelor:
  replicaCount: 3
  resources:
    limits:
      cpu: 1000m
      memory: 1Gi
    requests:
      cpu: 500m
      memory: 512Mi

frontdoor:
  replicaCount: 3
  service:
    type: LoadBalancer
  resources:
    limits:
      cpu: 1000m
      memory: 1Gi
    requests:
      cpu: 500m
      memory: 512Mi

podDisruptionBudget:
  enabled: true
  minAvailable: 2
```

### Example: Development Configuration

```yaml
chancelor:
  replicaCount: 1
  resources:
    limits:
      cpu: 200m
      memory: 256Mi
    requests:
      cpu: 50m
      memory: 64Mi

kubeOperator:
  replicaCount: 1
  resources:
    limits:
      cpu: 200m
      memory: 256Mi
    requests:
      cpu: 50m
      memory: 64Mi

frontdoor:
  replicaCount: 1
  service:
    type: NodePort
  resources:
    limits:
      cpu: 200m
      memory: 256Mi
    requests:
      cpu: 50m
      memory: 64Mi
```

## Notes

- The kube-operator requires RBAC permissions to manage Kubernetes resources. The chart creates a ClusterRole and ClusterRoleBinding with broad permissions. In production, you may want to restrict these permissions based on your specific needs.
- Frontdoor includes health check probes (liveness and readiness) that check the root path `/`.
- All services support custom environment variables through the `env` parameter in their respective sections.

## Troubleshooting

### Check Pod Status

```bash
kubectl get pods -l app.kubernetes.io/instance=degov
```

### View Logs

```bash
# Chancelor
kubectl logs -l app.kubernetes.io/component=chancelor

# Kube Operator
kubectl logs -l app.kubernetes.io/component=kube-operator

# Frontdoor
kubectl logs -l app.kubernetes.io/component=frontdoor
```

### Check Service

```bash
kubectl get svc -l app.kubernetes.io/instance=degov
```

