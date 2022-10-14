Authentication-Results: spf=pass (sender IP is 209.85.221.68)
 smtp.mailfrom=gmail.com; dkim=pass (signature was verified)
 header.d=gmail.com;dmarc=pass action=none header.from=gmail.com;compauth=pass
 reason=100
From: <alice@gmail.com>
To: <alice@hotmail.com>
