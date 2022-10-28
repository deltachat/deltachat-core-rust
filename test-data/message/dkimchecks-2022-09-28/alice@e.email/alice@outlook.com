ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@e.email>
Authentication-Results: mail3.ecloud.global;
	dkim=pass header.d=outlook.com header.s=selector1 header.b=MqNsAJKf;
	arc=pass ("microsoft.com:s=arcselector9901:i=1");
	dmarc=pass (policy=none) header.from=outlook.com;
	spf=pass (mail3.ecloud.global: domain of alice@outlook.com designates 40.92.66.84 as permitted sender) smtp.mailfrom=alice@outlook.com
