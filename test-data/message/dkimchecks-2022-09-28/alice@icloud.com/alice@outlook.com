Authentication-Results: bimi.icloud.com; bimi=skipped reason="insufficient dmarc"
Authentication-Results: dmarc.icloud.com; dmarc=pass header.from=outlook.com
Authentication-Results: dkim-verifier.icloud.com;
	dkim=pass (2048-bit key) header.d=outlook.com header.i=@outlook.com header.b=aRO3cX1y
Authentication-Results: spf.icloud.com; spf=pass (spf.icloud.com: domain of alice@outlook.com designates 40.92.66.68 as permitted sender) smtp.mailfrom=alice@outlook.com
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@icloud.com>
