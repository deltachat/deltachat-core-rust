Authentication-Results: spf=pass (sender IP is 213.182.54.12)
 smtp.mailfrom=mailo.com; dkim=pass (signature was verified)
 header.d=mailo.com;dmarc=pass action=none header.from=mailo.com;compauth=pass
 reason=100
From: <alice@mailo.com>
To: <alice@outlook.com>
