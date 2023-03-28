<br/>
<p align="center">
  <h3 align="center">Cloudflare Access Webhook Redirect</h3>

  <p align="center">
    <a href="https://github.com/Timmi6790/cloudflare-access-webhook-redirect/issues">Report Bug</a>
    .
    <a href="https://github.com/Timmi6790/cloudflare-access-webhook-redirect/issues">Request Feature</a>
  </p>
</p>

<div align="center">

![Docker Image Version (latest semver)](https://img.shields.io/docker/v/timmi6790/cloudflare-access-webhook-redirect)
![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/Timmi6790/cloudflare-access-webhook-redirect/build.yml)
![Issues](https://img.shields.io/github/issues/Timmi6790/cloudflare-access-webhook-redirect)
[![codecov](https://codecov.io/gh/Timmi6790/cloudflare-access-webhook-redirect/branch/main/graph/badge.svg?token=dDUZjsYmh2)](https://codecov.io/gh/Timmi6790/cloudflare-access-webhook-redirect)
![License](https://img.shields.io/github/license/Timmi6790/cloudflare-access-webhook-redirect)
[![wakatime](https://wakatime.com/badge/github/Timmi6790/cloudflare-access-webhook-redirect.svg)](https://wakatime.com/badge/github/Timmi6790/cloudflare-access-webhook-redirect)

</div>

## About The Project

A simple forward proxy to allow specified post paths to be forwarded through a cloudflare access protected endpoint.
A usage case would be github webhooks since they don't support custom headers.

### Installation - Helm chart

- [Helm chart](https://github.com/Timmi6790/helm-charts/tree/main/charts/cloudflare-access-webhook-redirect)


### Environment variables

| Environment    	                 | Required 	  | Description                         	                                             |
|----------------------------------|-------------|-----------------------------------------------------------------------------------|
| CLOUDFLARE.CLIENT_ID     	       | X	          | Cloudflare Access client id                        	                              |
| CLOUDFLARE.CLIENT_SECRET       	 | X         	 | Cloudflare Access client secret                     	                             |
| WEBHOOK.TARGET_BASE     	        | X	          | Forward target base                            	                                  |
| WEBHOOK.PATHS    	               | X	          | Allowed paths as regex seperated by `, `                           	              |
| SERVER.HOST 	                    | 	           | Server host [Default: 0.0.0.0]	                                                   |
| SERVER.PORT       	              | 	           | Server port [Default: 8080]                           	                           |
| SENTRY_DSN     	                 | 	           | Sentry DSN                          	                                             |
| LOG_LEVEL  	                     | 	           | Log level [FATAL, ERROR, WARN, INFO, DEBUG, TRACE, ALL]                         	 |

## License

See [LICENSE](https://github.com/Timmi6790/netcup-offer-bot/blob/main/LICENSE.md) for
more information.