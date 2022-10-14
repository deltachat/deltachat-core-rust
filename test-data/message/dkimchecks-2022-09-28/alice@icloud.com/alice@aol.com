Authentication-Results: bimi.icloud.com; bimi=none
Authentication-Results: dmarc.icloud.com; dmarc=pass header.from=aol.com
Authentication-Results: dkim-verifier.icloud.com;
	dkim=pass (2048-bit key) header.d=aol.com header.i=@aol.com header.b=XubAwo48
Authentication-Results: spf.icloud.com; spf=pass (spf.icloud.com: domain of alice@aol.com designates 87.248.110.84 as permitted sender) smtp.mailfrom=alice@aol.com
From: <alice@aol.com>
To: <alice@icloud.com>
