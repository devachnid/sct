#!/bin/sh
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Resolves the pieces of ../Caddyfile that can't safely be expressed as a
# Caddyfile {$VAR:default} placeholder, then execs Caddy.
#
# Why this wrapper exists rather than a single static Caddyfile:
#   - Caddy's {$VAR:default} substitution happens at Caddyfile PARSE time
#     (before the config is even built), not per-request, and a default value
#     containing a colon (e.g. ":80" as the fallback site address when DOMAIN
#     is unset) is ambiguous with Caddy's host:port address syntax. Plain
#     shell parameter expansion has no such problem.
#   - Caddy has no "only include this directive if an env var is set"
#     primitive. basic_auth in particular must be entirely ABSENT when auth
#     is not configured - a block with unsatisfiable dummy credentials would
#     lock every request out, which is the opposite of "optional".
# Both are solved the same way: write small snippet files that ../Caddyfile
# `import`s, then start Caddy. An empty snippet imports zero directives.

set -eu

mkdir -p /etc/caddy/snippets

# Site address: the real domain (triggers Caddy's automatic HTTPS/ACME), or
# a bare port for plain HTTP when no domain is configured (local/dev use, or
# running behind another TLS-terminating proxy).
export CADDY_SITE_ADDRESS="${DOMAIN:-:80}"

# Global options: an ACME account email, if given (recommended - Let's
# Encrypt sends cert-expiry notices to it - but not required).
if [ -n "${ACME_EMAIL:-}" ]; then
    printf 'email %s\n' "$ACME_EMAIL" > /etc/caddy/snippets/globals.caddy
else
    : > /etc/caddy/snippets/globals.caddy
fi

# HTTP basic auth: only when BOTH a username and a pre-hashed password are
# supplied. Terminology data is non-PHI and read-only, so this is opt-in -
# mainly abuse control on a publicly reachable endpoint.
if [ -n "${BASIC_AUTH_USER:-}" ] && [ -n "${BASIC_AUTH_HASH:-}" ]; then
    cat > /etc/caddy/snippets/auth.caddy <<EOF
basic_auth {
	$BASIC_AUTH_USER $BASIC_AUTH_HASH
}
EOF
else
    : > /etc/caddy/snippets/auth.caddy
fi

exec caddy run --config /etc/caddy/Caddyfile --adapter caddyfile
