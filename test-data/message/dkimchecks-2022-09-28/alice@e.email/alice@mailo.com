From: <alice@mailo.com>
To: <alice@e.email>
Authentication-Results: mail2.ecloud.global;
	dkim=pass header.d=mailo.com header.s=mailo header.b=HnhKNKUg;
	dmarc=pass (policy=none) header.from=mailo.com;
	spf=pass (mail2.ecloud.global: domain of alice@mailo.com designates 213.182.54.15 as permitted sender) smtp.mailfrom=alice@mailo.com
