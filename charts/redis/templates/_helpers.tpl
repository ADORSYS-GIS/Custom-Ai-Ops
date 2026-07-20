{{/*
Expand the name of the chart.
*/}}
{{- define "redis.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "redis.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Chart label values.
*/}}
{{- define "redis.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels.
*/}}
{{- define "redis.labels" -}}
helm.sh/chart: {{ include "redis.chart" . }}
{{ include "redis.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- with .Values.commonLabels }}
{{ toYaml . }}
{{- end }}
{{- end }}

{{/*
Selector labels.
*/}}
{{- define "redis.selectorLabels" -}}
app.kubernetes.io/name: {{ include "redis.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Master pod labels — include the selector labels plus a fixed role label.
*/}}
{{- define "redis.masterLabels" -}}
{{ include "redis.selectorLabels" . }}
app.kubernetes.io/component: redis
redis.io/role: master
{{- end }}

{{/*
Replica pod labels — include the selector labels plus a fixed role label.
*/}}
{{- define "redis.replicaLabels" -}}
{{ include "redis.selectorLabels" . }}
app.kubernetes.io/component: redis
redis.io/role: replica
{{- end }}

{{/*
Sentinel pod labels.
*/}}
{{- define "redis.sentinelLabels" -}}
{{ include "redis.selectorLabels" . }}
app.kubernetes.io/component: sentinel
{{- end }}

{{/*
Common annotations.
*/}}
{{- define "redis.annotations" -}}
{{- with .Values.commonAnnotations }}
{{ toYaml . }}
{{- end }}
{{- end }}

{{/*
Redis auth secret name.
*/}}
{{- define "redis.secretName" -}}
{{- if .Values.auth.existingSecret }}
{{- .Values.auth.existingSecret }}
{{- else }}
{{- include "redis.fullname" . }}-auth
{{- end }}
{{- end }}

{{/*
Redis password key.
*/}}
{{- define "redis.secretPasswordKey" -}}
{{- .Values.auth.secretKey | default "redis-password" }}
{{- end }}

{{/*
Redis master service name.
*/}}
{{- define "redis.primaryServiceName" -}}
{{ include "redis.fullname" . }}-primary
{{- end }}

{{/*
Redis headless service name (for sentinel discovery).
*/}}
{{- define "redis.headlessServiceName" -}}
{{ include "redis.fullname" . }}-headless
{{- end }}

{{/*
Sentinel master name (used by sentinel config).
*/}}
{{- define "redis.sentinelMasterName" -}}
mymaster
{{- end }}

{{/*
Namespace to deploy into: use .Release.Namespace by default,
but allow override via global.namespace (useful for subchart deployments).
*/}}
{{- define "redis.namespace" -}}
{{- .Values.global.namespace | default .Release.Namespace }}
{{- end }}
