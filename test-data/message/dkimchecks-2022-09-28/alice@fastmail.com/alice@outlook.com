ARC-Authentication-Results: i=2; mx3.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=fail smtp.helo=EUR01-VE1-obe.outbound.protection.outlook.com
    policy.ptr=mail-oln040092066024.outbound.protection.outlook.com;
    bimi=skipped (DMARC Policy is not at enforcement);
    arc=pass (as.1.microsoft.com=pass, ams.1.microsoft.com=pass)
    smtp.remote-ip=40.92.66.24;
    dkim=pass (2048-bit rsa key sha256) header.d=outlook.com
    header.i=@outlook.com header.b=Qx1vn7vt header.a=rsa-sha256
    header.s=selector1 x-bits=2048;
    dmarc=pass policy.published-domain-policy=none
    policy.published-subdomain-policy=quarantine
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=none,sp=quarantine,d=none,d.eval=none) policy.policy-from=p
    header.from=outlook.com;
    iprev=pass smtp.remote-ip=40.92.66.24
    (mail-oln040092066024.outbound.protection.outlook.com);
    spf=pass smtp.mailfrom=alice@outlook.com
    smtp.helo=EUR01-VE1-obe.outbound.protection.outlook.com
Authentication-Results: mx3.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=fail smtp.helo=EUR01-VE1-obe.outbound.protection.outlook.com
      policy.ptr=mail-oln040092066024.outbound.protection.outlook.com
Authentication-Results: mx3.messagingengine.com;
    bimi=skipped (DMARC Policy is not at enforcement)
Authentication-Results: mx3.messagingengine.com;
    arc=pass (as.1.microsoft.com=pass, ams.1.microsoft.com=pass)
      smtp.remote-ip=40.92.66.24
Authentication-Results: mx3.messagingengine.com;
    dkim=pass (2048-bit rsa key sha256) header.d=outlook.com
      header.i=@outlook.com header.b=Qx1vn7vt header.a=rsa-sha256
      header.s=selector1 x-bits=2048;
    dmarc=pass policy.published-domain-policy=none
      policy.published-subdomain-policy=quarantine
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=none,sp=quarantine,d=none,d.eval=none) policy.policy-from=p
      header.from=outlook.com;
    iprev=pass smtp.remote-ip=40.92.66.24
      (mail-oln040092066024.outbound.protection.outlook.com);
    spf=pass smtp.mailfrom=alice@outlook.com
      smtp.helo=EUR01-VE1-obe.outbound.protection.outlook.com
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@fastmail.com>
