Authentication-Results: spf=pass (sender IP is 77.238.176.99)
 smtp.mailfrom=yahoo.com; dkim=pass (signature was verified)
 header.d=yahoo.com;dmarc=pass action=none header.from=yahoo.com;compauth=pass
 reason=100
From: <alice@yahoo.com>
To: <alice@hotmail.com>
