From: <alice@yahoo.com>
To: <alice@e.email>
Authentication-Results: mail2.ecloud.global;
	dkim=pass header.d=yahoo.com header.s=s2048 header.b=mRSwanl2;
	dmarc=pass (policy=reject) header.from=yahoo.com;
	spf=pass (mail2.ecloud.global: domain of alice@yahoo.com designates 77.238.178.146 as permitted sender) smtp.mailfrom=alice@yahoo.com
