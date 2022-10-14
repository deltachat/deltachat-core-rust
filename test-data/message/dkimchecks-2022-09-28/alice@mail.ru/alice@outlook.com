ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@mail.ru>
Authentication-Results: mxs.mail.ru; spf=pass (mx222.i.mail.ru: domain of outlook.com designates 40.92.66.68 as permitted sender) smtp.mailfrom=alice@outlook.com smtp.helo=EUR01-VE1-obe.outbound.protection.outlook.com;
	 dkim=pass header.d=outlook.com; dmarc=pass header.from=alice@outlook.com
