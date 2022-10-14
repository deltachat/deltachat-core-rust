From: <alice@icloud.com>
To: <alice@e.email>
Authentication-Results: mail3.ecloud.global;
	dkim=pass header.d=icloud.com header.s=1a1hai header.b=enuQcpfH;
	dmarc=pass (policy=quarantine) header.from=icloud.com;
	spf=pass (mail3.ecloud.global: domain of alice@icloud.com designates 17.57.155.16 as permitted sender) smtp.mailfrom=alice@icloud.com
