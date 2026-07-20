{{/*
Expand the name of the chart.
*/}}
{{- define "llm-d.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "llm-d.fullname" -}}
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
{{- define "llm-d.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels.
*/}}
{{- define "llm-d.labels" -}}
helm.sh/chart: {{ include "llm-d.chart" . }}
{{ include "llm-d.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- with .Values.commonLabels }}
{{ toYaml . }}
{{- end }}
{{- end }}

{{/*
Selector labels.
*/}}
{{- define "llm-d.selectorLabels" -}}
app.kubernetes.io/name: {{ include "llm-d.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Common annotations.
*/}}
{{- define "llm-d.annotations" -}}
{{- with .Values.commonAnnotations }}
{{ toYaml . }}
{{- end }}
{{- end }}

{{/*
Service account name.
*/}}
{{- define "llm-d.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "llm-d.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Router fullname.
*/}}
{{- define "llm-d.router.fullname" -}}
{{- printf "%s-router" (include "llm-d.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
KV-Cache Indexer fullname.
*/}}
{{- define "llm-d.indexer.fullname" -}}
{{- printf "%s-indexer" (include "llm-d.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
EPP config JSON for the ConfigMap.
*/}}
{{- define "llm-d.eppConfig" -}}
{{- $epp := .Values.router.epp -}}
{
  "plugin": "{{ $epp.plugin }}",
  "discovery": "{{ $epp.discovery }}",
  "minReplicas": {{ $epp.minReplicas }},
  "scoringWeights": {
    "cacheHit": {{ $epp.scoringWeights.cacheHit }},
    "queueDepth": {{ $epp.scoringWeights.queueDepth }},
    "gpuUtilization": {{ $epp.scoringWeights.gpuUtilization }}
  },
  "maxInflightPerReplica": {{ $epp.maxInflightPerReplica }}
}
{{- end }}