Authentication-Results: bimi.icloud.com; bimi=skipped reason="insufficient dmarc"
Authentication-Results: dmarc.icloud.com; dmarc=pass header.from=gmail.com
Authentication-Results: dkim-verifier.icloud.com;
	dkim=pass (2048-bit key) header.d=gmail.com header.i=@gmail.com header.b=HYnahEVt
Authentication-Results: spf.icloud.com; spf=pass (spf.icloud.com: domain of alice@gmail.com designates 209.85.221.67 as permitted sender) smtp.mailfrom=alice@gmail.com
From: <alice@gmail.com>
To: <alice@icloud.com>
