ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@mail.ru>
Authentication-Results: mxs.mail.ru; spf=pass (mx200.i.mail.ru: domain of hotmail.com designates 40.92.73.37 as permitted sender) smtp.mailfrom=alice@hotmail.com smtp.helo=EUR04-HE1-obe.outbound.protection.outlook.com;
	 dkim=pass header.d=hotmail.com; dmarc=pass header.from=alice@hotmail.com
