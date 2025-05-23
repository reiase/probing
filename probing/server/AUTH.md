# Probe Server Authentication

The probe server now supports a simple token-based authentication system. When the authentication token environment variable is set, API endpoints will require authentication to access.

## Enabling Authentication

Set the `PROBING_AUTH_TOKEN` environment variable to enable authentication:

```bash
export PROBING_AUTH_TOKEN="your-secret-token"
```

If this environment variable is not set or is empty, authentication will not be enabled and all endpoints will be publicly accessible.

## Accessing the API with Authentication

Once authentication is enabled, clients must provide a valid token in the request headers to access the API. There are two ways to provide the token:

### 1. Using the Authorization Header

```
Authorization: Bearer your-secret-token
```

### 2. Using the X-Probing-Token Header

```
X-Probing-Token: your-secret-token
```

## Security Considerations

- Tokens are transmitted in plain text, so HTTPS should be considered for production environments.
- For highly sensitive applications, consider implementing a more sophisticated authentication system.
- This basic authentication is primarily intended to prevent unauthorized access to the probe server.
