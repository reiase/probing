# Probe Server Authentication

The probe server now supports a simple token-based authentication system. When the authentication token environment variable is set, API endpoints will require authentication to access.

## Enabling Authentication

Set the `PROBING_AUTH_TOKEN` environment variable to enable authentication:

```bash
export PROBING_AUTH_TOKEN="your-secret-token"
```

You can also optionally set the username (default is "admin") and the realm (default is "Probe Server"):

```bash
export PROBING_AUTH_USERNAME="your-username"
export PROBING_AUTH_REALM="Your Custom Realm Name"
```

If the auth token environment variable is not set or is empty, authentication will not be enabled and all endpoints will be publicly accessible.

## Accessing the API with Authentication

Once authentication is enabled, clients must provide valid credentials to access the API. There are three ways to authenticate:

### 1. Browser Login Prompt (Basic Auth)

When accessing protected endpoints through a browser, users will automatically see a login dialog. Enter the username (default: "admin") and use the token as the password.

### 2. Using the Authorization Header with Basic Auth

For programmatic access, you can use HTTP Basic Authentication:

```
Authorization: Basic base64(username:token)
```

Where `base64(username:token)` is the Base64 encoding of `username:token`.

### 3. Using the Bearer Token

```
Authorization: Bearer your-secret-token
```

### 4. Using the X-Probing-Token Header

```
X-Probing-Token: your-secret-token
```

## Public Paths

Even when authentication is enabled, the following paths remain publicly accessible by default:

- `/` (home page)
- `/index.html`
- `/static/` (all static resources)
- `/favicon*` (website icons)

## Security Considerations

- Tokens are transmitted in plain text, so HTTPS should be considered for production environments.
- For highly sensitive applications, consider implementing a more sophisticated authentication system.
- This basic authentication is primarily intended to prevent unauthorized access to the probe server.
