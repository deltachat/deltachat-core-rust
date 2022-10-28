Authentication-Results: spf=pass (sender IP is 77.238.178.97)
 smtp.mailfrom=aol.com; dkim=pass (signature was verified)
 header.d=aol.com;dmarc=pass action=none header.from=aol.com;compauth=pass
 reason=100
From: <alice@aol.com>
To: <alice@hotmail.com>
