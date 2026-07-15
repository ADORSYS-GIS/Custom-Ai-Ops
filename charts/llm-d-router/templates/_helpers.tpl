{{/*
Expand the name of the chart.
*/}}
{{- define "llm-d-router.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "llm-d-router.fullname" -}}
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
Create chart name and version as used by the chart label.
*/}}
{{- define "llm-d-router.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "llm-d-router.labels" -}}
helm.sh/chart: {{ include "llm-d-router.chart" . }}
{{ include "llm-d-router.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/component: router
{{- end }}

{{/*
Selector labels
*/}}
{{- define "llm-d-router.selectorLabels" -}}
app.kubernetes.io/name: {{ include "llm-d-router.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Proxy labels
*/}}
{{- define "llm-d-router.proxyLabels" -}}
{{ include "llm-d-router.labels" . }}
app.kubernetes.io/subcomponent: proxy
{{- end }}

{{/*
EPP labels
*/}}
{{- define "llm-d-router.eppLabels" -}}
{{ include "llm-d-router.labels" . }}
app.kubernetes.io/subcomponent: epp
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "llm-d-router.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "llm-d-router.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Proxy selector labels
*/}}
{{- define "llm-d-router.proxySelectorLabels" -}}
{{ include "llm-d-router.selectorLabels" . }}
app.kubernetes.io/subcomponent: proxy
{{- end }}

{{/*
EPP selector labels
*/}}
{{- define "llm-d-router.eppSelectorLabels" -}}
{{ include "llm-d-router.selectorLabels" . }}
app.kubernetes.io/subcomponent: epp
{{- end }}
