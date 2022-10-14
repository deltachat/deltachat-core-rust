ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@e.email>
Authentication-Results: mail3.ecloud.global;
	dkim=pass header.d=hotmail.com header.s=selector1 header.b="ECc21y/J";
	arc=pass ("microsoft.com:s=arcselector9901:i=1");
	dmarc=pass (policy=none) header.from=hotmail.com;
	spf=pass (mail3.ecloud.global: domain of alice@hotmail.com designates 40.92.68.54 as permitted sender) smtp.mailfrom=alice@hotmail.com
