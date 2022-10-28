Authentication-Results: bimi.icloud.com; bimi=skipped reason="insufficient dmarc"
Authentication-Results: dmarc.icloud.com; dmarc=pass header.from=mailo.com
Authentication-Results: dkim-verifier.icloud.com;
	dkim=pass (1024-bit key) header.d=mailo.com header.i=@mailo.com header.b=iBgqeTn7
Authentication-Results: spf.icloud.com; spf=pass (spf.icloud.com: domain of alice@mailo.com designates 213.182.54.11 as permitted sender) smtp.mailfrom=alice@mailo.com
From: <alice@mailo.com>
To: <alice@icloud.com>
