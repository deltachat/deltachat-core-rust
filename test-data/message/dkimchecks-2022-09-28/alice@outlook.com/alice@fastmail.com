Authentication-Results: spf=pass (sender IP is 66.111.4.28)
 smtp.mailfrom=fastmail.com; dkim=pass (signature was verified)
 header.d=fastmail.com;dmarc=pass action=none
 header.from=fastmail.com;compauth=pass reason=100
From: <alice@fastmail.com>
To: <alice@outlook.com>
