From: <alice@aol.com>
To: <alice@e.email>
Authentication-Results: mail3.ecloud.global;
	dkim=pass header.d=aol.com header.s=a2048 header.b=HlBq3Lmt;
	dmarc=pass (policy=reject) header.from=aol.com;
	spf=pass (mail3.ecloud.global: domain of alice@aol.com designates 77.238.178.146 as permitted sender) smtp.mailfrom=alice@aol.com
